//! Fixed-point money amounts, stored as an integer count of the currency's minor unit
//! (e.g. cents) so arithmetic is exact — see D-008. `Amount` itself carries no scale:
//! since Slice 2, scale is a per-currency property (`currency ETB { scale = 2 }`), so
//! lowering a literal (`from_literal`) and printing a value (`format`) both take the
//! scale of whichever currency the amount belongs to, rather than assuming one.
//!
//! `Amount` itself is signed (a running ledger balance needs `debits - credits`), but
//! a `Money<C>` *literal* is non-negative by construction (D-014): the lexer has no
//! `-` token in numeric-literal position, so a negative amount cannot be written.

use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Amount(i64);

/// Why a decimal literal couldn't be lowered into an `Amount` (Fix checkpoint B: found
/// by fuzzing with an oversized literal, which overflowed `i64` instead of producing a
/// diagnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralError {
    /// The literal has more fraction digits than the currency's scale allows; carries
    /// the actual count.
    TooManyFractionDigits(u32),
    /// The literal's magnitude (or its magnitude once scaled to the currency's minor
    /// unit) doesn't fit an `i64`.
    OutOfRange,
}

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

    /// Lowers a decimal literal's raw text — lexer-validated as digits, at most one
    /// `.`, and at least one fraction digit if a `.` is present — into `scale`'s minor
    /// unit. The caller (`typeck`) names the currency and its scale in the diagnostic
    /// for `TooManyFractionDigits`, since this module doesn't know either (D-024).
    pub fn from_literal(text: &str, scale: u32) -> Result<Amount, LiteralError> {
        let (numerator, frac_digits) =
            parse_fixed_point(text).ok_or(LiteralError::OutOfRange)?;
        if frac_digits > scale {
            return Err(LiteralError::TooManyFractionDigits(frac_digits));
        }
        let scale_factor = 10i64.checked_pow(scale - frac_digits).ok_or(LiteralError::OutOfRange)?;
        numerator.checked_mul(scale_factor).map(Amount).ok_or(LiteralError::OutOfRange)
    }

    /// Renders this amount at `scale` fraction digits (e.g. `scale = 2` -> `"45.00"`,
    /// `scale = 0` -> `"45"`, no decimal point at all).
    pub fn format(self, scale: u32) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        if scale == 0 {
            format!("{sign}{abs}")
        } else {
            let base = 10u64.pow(scale);
            format!("{sign}{}.{:0width$}", abs / base, abs % base, width = scale as usize)
        }
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

/// Splits a lexer-validated decimal literal's raw text into a single integer
/// numerator and its fraction-digit count, e.g. `"57.20"` -> `Ok((5720, 2))`. Shared by
/// `Amount::from_literal` (which then needs a currency's scale to finish lowering)
/// and a `rate`'s own literal (`resolve::resolve_rates`, Slice 3), which has no scale
/// to validate against — a rate's precision is whatever the author wrote.
///
/// `None` means the literal's magnitude doesn't fit an `i64`. The lexer only validates
/// a literal's *shape* (digits, at most one `.`) — nothing bounds how many digits a
/// user can type, so a literal with enough digits genuinely overflows this module's
/// fixed-point representation. Every caller must handle this as an ordinary diagnostic
/// (`E_AMOUNT_OUT_OF_RANGE`), not treat it as unreachable.
pub fn parse_fixed_point(text: &str) -> Option<(i64, u32)> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let frac_digits = frac.len() as u32;
    let whole: i64 = whole.parse().ok()?;
    let frac_value: i64 = if frac.is_empty() { 0 } else { frac.parse().ok()? };
    let scaled_whole = whole.checked_mul(10i64.checked_pow(frac_digits)?)?;
    Some((scaled_whole.checked_add(frac_value)?, frac_digits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_pads_to_scale() {
        assert_eq!(Amount::from_cents(4500).format(2), "45.00");
        assert_eq!(Amount::from_cents(5).format(2), "0.05");
        assert_eq!(Amount::from_cents(-150).format(2), "-1.50");
    }

    #[test]
    fn format_at_scale_zero_has_no_decimal_point() {
        assert_eq!(Amount::from_cents(45).format(0), "45");
        assert_eq!(Amount::from_cents(-45).format(0), "-45");
    }

    #[test]
    fn add_sub_neg() {
        let a = Amount::from_cents(4500);
        let b = Amount::from_cents(1000);
        assert_eq!((a + b).cents(), 5500);
        assert_eq!((a - b).cents(), 3500);
        assert_eq!((-a).cents(), -4500);
    }

    #[test]
    fn from_literal_at_matching_scale() {
        assert_eq!(Amount::from_literal("45", 2).unwrap().cents(), 4500);
        assert_eq!(Amount::from_literal("45.5", 2).unwrap().cents(), 4550);
        assert_eq!(Amount::from_literal("45.00", 2).unwrap().cents(), 4500);
        assert_eq!(Amount::from_literal("0.01", 2).unwrap().cents(), 1);
    }

    #[test]
    fn from_literal_at_scale_zero() {
        assert_eq!(Amount::from_literal("45", 0).unwrap().cents(), 45);
    }

    #[test]
    fn from_literal_rejects_too_many_fraction_digits() {
        assert_eq!(Amount::from_literal("45.123", 2), Err(LiteralError::TooManyFractionDigits(3)));
    }

    #[test]
    fn parse_fixed_point_splits_numerator_and_frac_digits() {
        assert_eq!(parse_fixed_point("57.20"), Some((5720, 2)));
        assert_eq!(parse_fixed_point("45"), Some((45, 0)));
        assert_eq!(parse_fixed_point("0.001"), Some((1, 3)));
    }

    #[test]
    fn parse_fixed_point_rejects_a_literal_too_large_for_i64() {
        // Fix checkpoint B: fuzzing found this panicking via `.expect()`/`.unwrap()`
        // instead of reporting a diagnostic (a 100k-digit literal is lexer-valid shape,
        // but no magnitude fits `i64`).
        assert_eq!(parse_fixed_point(&"9".repeat(100)), None);
    }

    #[test]
    fn from_literal_rejects_an_amount_too_large_for_i64() {
        assert_eq!(Amount::from_literal(&"9".repeat(30), 2), Err(LiteralError::OutOfRange));
    }

    #[test]
    fn from_literal_rejects_a_scale_so_large_it_overflows_i64() {
        // A literal that's fine on its own can still overflow once scaled up to an
        // absurdly large currency scale.
        assert_eq!(Amount::from_literal("1", 30), Err(LiteralError::OutOfRange));
    }
}
