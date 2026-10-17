//! Exact money arithmetic. All amounts are stored as `i64` **centimes**
//! (1 DZD = 100 centimes). Floats are never used for money.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// An amount of money, in centimes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Money(i64);

/// Divide rounding half away from zero (used for tax / percentage math).
#[inline]
fn div_round_half_up(n: i64, d: i64) -> i64 {
    debug_assert!(d > 0, "divisor must be positive");
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -(((-n) + d / 2) / d)
    }
}

impl Money {
    pub const ZERO: Money = Money(0);

    #[inline]
    pub const fn from_centimes(c: i64) -> Self {
        Money(c)
    }

    #[inline]
    pub const fn centimes(self) -> i64 {
        self.0
    }

    /// Build from whole dinars.
    #[inline]
    pub const fn from_dzd(d: i64) -> Self {
        Money(d * 100)
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[inline]
    pub const fn abs(self) -> Self {
        Money(self.0.abs())
    }

    /// Multiply by an integer quantity, saturating on overflow.
    #[inline]
    pub fn mul_qty(self, qty: i64) -> Self {
        Money(self.0.saturating_mul(qty))
    }

    /// Apply a rate expressed in basis points (1% = 100 bp), rounding half up.
    #[inline]
    pub fn percent_bp(self, bp: u32) -> Self {
        Money(div_round_half_up(self.0.saturating_mul(bp as i64), 10_000))
    }

    #[inline]
    pub fn clamp_to(self, min: Money, max: Money) -> Self {
        Money(self.0.clamp(min.0, max.0))
    }

    /// Format as Algerian dinar, e.g. `1 234,56 DA`.
    pub fn format_dzd(self) -> String {
        format!("{} DA", self.format_amount())
    }

    /// Format the numeric part only, e.g. `1 234,56`.
    pub fn format_amount(self) -> String {
        let neg = self.0 < 0;
        let a = self.0.unsigned_abs();
        let dinars = a / 100;
        let cents = a % 100;
        let grouped = group_thousands(dinars);
        format!("{}{},{:02}", if neg { "-" } else { "" }, grouped, cents)
    }

    /// Parse a user string such as `1234,56`, `1 234.56` or `1200` into centimes.
    pub fn parse(input: &str) -> Option<Money> {
        let cleaned: String = input
            .chars()
            .filter(|c| *c != ' ' && *c != '\u{00a0}' && *c != '\u{202f}')
            .collect();
        if cleaned.is_empty() {
            return None;
        }
        let normalized = cleaned.replace(',', ".");
        // Only digits, at most one dot, optional leading minus.
        let neg = normalized.starts_with('-');
        let body = normalized.trim_start_matches('-');
        if body.is_empty() || body.matches('.').count() > 1 {
            return None;
        }
        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let int_val: i64 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().ok()?
        };
        let mut frac_str: String = frac_part.chars().take(2).collect();
        while frac_str.len() < 2 {
            frac_str.push('0');
        }
        let frac_val: i64 = frac_str.parse().ok()?;
        let centimes = int_val.checked_mul(100)?.checked_add(frac_val)?;
        Some(Money(if neg { -centimes } else { centimes }))
    }
}

fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('\u{202f}'); // narrow no-break space
        }
        out.push(*b as char);
    }
    out
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_dzd())
    }
}

impl Add for Money {
    type Output = Money;
    #[inline]
    fn add(self, rhs: Money) -> Money {
        Money(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Money {
    #[inline]
    fn add_assign(&mut self, rhs: Money) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Money {
    type Output = Money;
    #[inline]
    fn sub(self, rhs: Money) -> Money {
        Money(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Money {
    #[inline]
    fn sub_assign(&mut self, rhs: Money) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Mul<i64> for Money {
    type Output = Money;
    #[inline]
    fn mul(self, rhs: i64) -> Money {
        Money(self.0.saturating_mul(rhs))
    }
}

impl Neg for Money {
    type Output = Money;
    #[inline]
    fn neg(self) -> Money {
        Money(-self.0)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_uses_algerian_conventions() {
        assert_eq!(
            Money::from_centimes(123456).format_dzd(),
            "1\u{202f}234,56 DA"
        );
        assert_eq!(Money::from_dzd(0).format_dzd(), "0,00 DA");
        assert_eq!(Money::from_centimes(5).format_dzd(), "0,05 DA");
        assert_eq!(
            Money::from_centimes(-123456).format_dzd(),
            "-1\u{202f}234,56 DA"
        );
        assert_eq!(
            Money::from_dzd(1_000_000).format_dzd(),
            "1\u{202f}000\u{202f}000,00 DA"
        );
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        // 19% of 1,00 DZD (100c) = 19c exactly
        assert_eq!(
            Money::from_centimes(100).percent_bp(1900),
            Money::from_centimes(19)
        );
        // 9% of 5c = 0.45c -> 0
        assert_eq!(Money::from_centimes(5).percent_bp(900), Money::ZERO);
        // 9% of 50c = 4.5c -> 5 (half away from zero)
        assert_eq!(
            Money::from_centimes(50).percent_bp(900),
            Money::from_centimes(5)
        );
        // 19% of 105c = 19.95 -> 20
        assert_eq!(
            Money::from_centimes(105).percent_bp(1900),
            Money::from_centimes(20)
        );
    }

    #[test]
    fn parse_handles_local_format() {
        assert_eq!(Money::parse("1234,56"), Some(Money::from_centimes(123456)));
        assert_eq!(Money::parse("1 234.56"), Some(Money::from_centimes(123456)));
        assert_eq!(Money::parse("1200"), Some(Money::from_dzd(1200)));
        assert_eq!(Money::parse("12,5"), Some(Money::from_centimes(1250)));
        assert_eq!(Money::parse("abc"), None);
        assert_eq!(Money::parse(""), None);
        assert_eq!(Money::parse("1.2.3"), None);
    }

    #[test]
    fn arithmetic_saturates_rather_than_panics() {
        let big = Money::from_centimes(i64::MAX);
        assert_eq!((big + Money::from_centimes(10)).centimes(), i64::MAX);
    }
}
