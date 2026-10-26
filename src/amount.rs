//! Exact money as ten-thousandths in an `i64`, with its decimal parser and four-place display.

use std::fmt;
use std::str::FromStr;

const SCALE: i64 = 10_000;
const SCALE_U64: u64 = SCALE.unsigned_abs();
const FRACTION_PLACES: [i64; 4] = [1000, 100, 10, 1];

/// Amount is a money value held as a count of ten-thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    Empty,
    Malformed,
    TooManyDecimals,
    Overflow,
}

impl Amount {
    pub const ZERO: Amount = Amount(0);

    pub const fn from_units(units: i64) -> Amount {
        Amount(units)
    }

    pub const fn units(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_add(rhs.0).map(Amount)
    }

    pub fn checked_sub(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_sub(rhs.0).map(Amount)
    }
}

fn digit(c: char) -> Option<i64> {
    c.to_digit(10).map(i64::from)
}

impl FromStr for Amount {
    type Err = AmountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(AmountError::Empty);
        }
        let only_digits_and_dot = s.chars().all(|c| c.is_ascii_digit() || c == '.');
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        if !only_digits_and_dot || !has_digit || s.matches('.').count() > 1 {
            return Err(AmountError::Malformed);
        }
        let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
        if fraction.len() > FRACTION_PLACES.len() {
            return Err(AmountError::TooManyDecimals);
        }
        let whole_units = whole
            .chars()
            .try_fold(0_i64, |acc, c| acc.checked_mul(10)?.checked_add(digit(c)?))
            .ok_or(AmountError::Overflow)?;
        let scaled = whole_units.checked_mul(SCALE).ok_or(AmountError::Overflow)?;
        let fraction_units = fraction
            .chars()
            .zip(FRACTION_PLACES)
            .try_fold(0_i64, |acc, (c, place)| acc.checked_add(digit(c)?.checked_mul(place)?))
            .ok_or(AmountError::Overflow)?;
        scaled.checked_add(fraction_units).map(Amount).ok_or(AmountError::Overflow)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        write!(f, "{sign}{}.{:04}", magnitude / SCALE_U64, magnitude % SCALE_U64)
    }
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Empty => "amount is empty",
            Self::Malformed => "amount is malformed",
            Self::TooManyDecimals => "amount has more than four decimal places",
            Self::Overflow => "amount is out of range",
        };
        f.write_str(text)
    }
}

impl std::error::Error for AmountError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_table() {
        let cases: &[(&str, &str, Result<i64, AmountError>)] = &[
            ("whole", "1", Ok(10000)),
            ("one_decimal", "1.0", Ok(10000)),
            ("four_decimals", "1.5000", Ok(15000)),
            ("smallest", "0.0001", Ok(1)),
            ("zero_plain", "0", Ok(0)),
            ("zero_padded", "0.0000", Ok(0)),
            ("trailing_dot", "1.", Ok(10000)),
            ("leading_dot", ".5", Ok(5000)),
            ("leading_zeros", "000000000000000000000000000001", Ok(10000)),
            ("max_value", "922337203685477.5807", Ok(i64::MAX)),
            ("empty", "", Err(AmountError::Empty)),
            ("dot_only", ".", Err(AmountError::Malformed)),
            ("plus_sign", "+1", Err(AmountError::Malformed)),
            ("plus_zero", "+0", Err(AmountError::Malformed)),
            ("minus_sign", "-1", Err(AmountError::Malformed)),
            ("minus_zero", "-0", Err(AmountError::Malformed)),
            ("exponent", "1e3", Err(AmountError::Malformed)),
            ("comma_decimal", "1,5", Err(AmountError::Malformed)),
            ("nan", "NaN", Err(AmountError::Malformed)),
            ("inf", "inf", Err(AmountError::Malformed)),
            ("letters", "abc", Err(AmountError::Malformed)),
            ("two_dots", "1.2.3", Err(AmountError::Malformed)),
            ("underscore", "1_000", Err(AmountError::Malformed)),
            ("leading_space", " 2.5", Err(AmountError::Malformed)),
            ("trailing_space", "2.5 ", Err(AmountError::Malformed)),
            ("inner_space", "1 .5", Err(AmountError::Malformed)),
            ("inner_tab", "1\t5", Err(AmountError::Malformed)),
            ("nbsp", "1\u{a0}5", Err(AmountError::Malformed)),
            ("five_decimals", "0.00001", Err(AmountError::TooManyDecimals)),
            ("trailing_zero_fifth", "1.50000", Err(AmountError::TooManyDecimals)),
            ("bad_char_beats_decimals", "1.23456x", Err(AmountError::Malformed)),
            ("one_over_max", "922337203685477.5808", Err(AmountError::Overflow)),
            ("whole_overflow", "922337203685478", Err(AmountError::Overflow)),
            ("huge", "99999999999999999999999999", Err(AmountError::Overflow)),
        ];

        let mut failures = Vec::new();
        for (name, input, expected) in cases {
            let got = input.parse::<Amount>().map(Amount::units);
            if got != *expected {
                failures.push(format!("{name}: input {input:?}: got {got:?}, want {expected:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn display_table() {
        let cases: &[(&str, i64, &str)] = &[
            ("zero", 0, "0.0000"),
            ("one_unit", 1, "0.0001"),
            ("five_units", 5, "0.0005"),
            ("minus_one_unit", -1, "-0.0001"),
            ("minus_half_hundredth", -500, "-0.0500"),
            ("minus_eight", -80000, "-8.0000"),
            ("one", 10000, "1.0000"),
            ("one_and_half", 15000, "1.5000"),
            ("just_below_ten", 99999, "9.9999"),
            ("ten", 100000, "10.0000"),
            ("max", i64::MAX, "922337203685477.5807"),
            ("min", i64::MIN, "-922337203685477.5808"),
        ];

        let mut failures = Vec::new();
        for (name, units, expected) in cases {
            let got = Amount::from_units(*units).to_string();
            if got != *expected {
                failures.push(format!("{name}: units {units}: got {got:?}, want {expected:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn checked_ops_table() {
        let cases: &[(&str, &str, i64, i64, Option<i64>)] = &[
            ("add_normal", "add", 10000, 5000, Some(15000)),
            ("add_zero", "add", i64::MAX, 0, Some(i64::MAX)),
            ("add_overflow", "add", i64::MAX, 1, None),
            ("add_to_negative", "add", i64::MIN, i64::MAX, Some(-1)),
            ("sub_normal", "sub", 15000, 5000, Some(10000)),
            ("sub_goes_negative", "sub", 1, 2, Some(-1)),
            ("sub_to_min", "sub", -i64::MAX, 1, Some(i64::MIN)),
            ("sub_underflow", "sub", i64::MIN, 1, None),
            ("sub_overflow_positive", "sub", i64::MAX, -1, None),
        ];

        let mut failures = Vec::new();
        for (name, op, a, b, expected) in cases {
            let lhs = Amount::from_units(*a);
            let rhs = Amount::from_units(*b);
            let got = match *op {
                "add" => lhs.checked_add(rhs),
                _ => lhs.checked_sub(rhs),
            }
            .map(Amount::units);
            if got != *expected {
                failures.push(format!("{name}: {op} {a} and {b}: got {got:?}, want {expected:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn round_trip_table() {
        let cases: &[(&str, i64)] = &[
            ("zero", 0),
            ("one_unit", 1),
            ("just_below_one", 9999),
            ("one", 10000),
            ("one_and_half", 15000),
            ("max", i64::MAX),
        ];

        let mut failures = Vec::new();
        for (name, units) in cases {
            let amount = Amount::from_units(*units);
            let text = amount.to_string();
            let got = text.parse::<Amount>();
            if got != Ok(amount) {
                failures.push(format!("{name}: text {text:?}: got {got:?}, want Ok({amount:?})"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
