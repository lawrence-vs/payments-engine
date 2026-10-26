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
mod tests;
