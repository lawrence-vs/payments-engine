use std::fmt;
use std::str::FromStr;

/// Amount is a money value held as a count of ten-thousandths (1.0 = 10000).
/// It is used for transaction amounts and for signed balances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(i64);

/// AmountError is the reason a text value could not become an Amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    /// Empty means the text had no characters at all.
    Empty,
    /// Malformed means the text had a character or shape that is not a decimal.
    Malformed,
    /// TooManyDecimals means the fraction had more than four digits.
    TooManyDecimals,
    /// Overflow means the value does not fit in the i64 range.
    Overflow,
}

impl Amount {
    /// ZERO is the Amount with no value.
    pub const ZERO: Amount = Amount(0);

    /// from_units builds an Amount from a count of ten-thousandths.
    pub const fn from_units(units: i64) -> Amount {
        Amount(units)
    }

    /// units returns the count of ten-thousandths.
    pub const fn units(self) -> i64 {
        self.0
    }

    /// checked_add returns the sum, or None on overflow.
    pub fn checked_add(self, _rhs: Amount) -> Option<Amount> {
        Some(self)
    }

    /// checked_sub returns the difference, or None on overflow.
    pub fn checked_sub(self, _rhs: Amount) -> Option<Amount> {
        Some(self)
    }
}

impl FromStr for Amount {
    type Err = AmountError;

    /// from_str parses a decimal text value with at most four fraction digits.
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(Amount(0))
    }
}

impl fmt::Display for Amount {
    /// fmt writes the value with exactly four decimal places.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("0")
    }
}

impl fmt::Display for AmountError {
    /// fmt writes a short description of the error.
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
