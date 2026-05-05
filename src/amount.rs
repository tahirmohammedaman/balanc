//! Fixed-point money amounts. Scale is hardcoded at 2 decimal digits until Slice 2
//! makes it per-currency. Stored as integer cents so arithmetic is exact — see D-008.
//!
//! `Amount` itself is signed (a running ledger balance needs `debits - credits`), but
//! a `Money<C>` *literal* is non-negative by construction (D-014): the lexer has no
//! `-` token in numeric-literal position, so a negative amount cannot be written.

use std::fmt;
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Amount(i64);

impl Amount {
    pub const ZERO: Amount = Amount(0);

    pub fn from_cents(cents: i64) -> Self {
        Amount(cents)
    }

    pub fn cents(self) -> i64 {
        self.0
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl Add for Amount {
    type Output = Amount;
    fn add(self, rhs: Amount) -> Amount {
        Amount(
            self.0
                .checked_add(rhs.0)
                .unwrap_or_else(|| panic!("internal error: Amount overflow on add")),
        )
    }
}

impl Sub for Amount {
    type Output = Amount;
    fn sub(self, rhs: Amount) -> Amount {
        Amount(
            self.0
                .checked_sub(rhs.0)
                .unwrap_or_else(|| panic!("internal error: Amount overflow on sub")),
        )
    }
}

impl Neg for Amount {
    type Output = Amount;
    fn neg(self) -> Amount {
        Amount(
            self.0
                .checked_neg()
                .unwrap_or_else(|| panic!("internal error: Amount overflow on neg")),
        )
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_pads_cents() {
        assert_eq!(Amount::from_cents(4500).to_string(), "45.00");
        assert_eq!(Amount::from_cents(5).to_string(), "0.05");
        assert_eq!(Amount::from_cents(-150).to_string(), "-1.50");
    }

    #[test]
    fn add_sub_neg() {
        let a = Amount::from_cents(4500);
        let b = Amount::from_cents(1000);
        assert_eq!((a + b).cents(), 5500);
        assert_eq!((a - b).cents(), 3500);
        assert_eq!((-a).cents(), -4500);
    }
}
