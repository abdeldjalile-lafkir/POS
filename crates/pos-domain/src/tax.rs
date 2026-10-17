//! Algerian TVA (VAT) rates, stored as basis points (1% = 100 bp).

use crate::money::Money;
use serde::{Deserialize, Serialize};

/// A tax rate in basis points, e.g. 1900 = 19%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaxRate(u32);

impl TaxRate {
    /// Standard Algerian TVA rate: 19%.
    pub const TVA_19: TaxRate = TaxRate(1900);
    /// Reduced Algerian TVA rate: 9%.
    pub const TVA_9: TaxRate = TaxRate(900);
    /// Exempt / zero-rated.
    pub const TVA_0: TaxRate = TaxRate(0);

    #[inline]
    pub const fn from_bp(bp: u32) -> Self {
        TaxRate(bp)
    }

    #[inline]
    pub const fn bp(self) -> u32 {
        self.0
    }

    /// Human label, e.g. `19%`, `9%`, `7.5%`.
    pub fn label(self) -> String {
        if self.0.is_multiple_of(100) {
            format!("{}%", self.0 / 100)
        } else {
            format!("{},{:02}%", self.0 / 100, self.0 % 100)
        }
    }

    /// Tax amount for a tax-exclusive (HT) base.
    #[inline]
    pub fn tax_on(self, base: Money) -> Money {
        base.percent_bp(self.0)
    }

    /// The three rates the UI offers.
    pub const ALL: [TaxRate; 3] = [TaxRate::TVA_19, TaxRate::TVA_9, TaxRate::TVA_0];
}

impl Default for TaxRate {
    fn default() -> Self {
        TaxRate::TVA_19
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(TaxRate::TVA_19.label(), "19%");
        assert_eq!(TaxRate::TVA_9.label(), "9%");
        assert_eq!(TaxRate::TVA_0.label(), "0%");
        assert_eq!(TaxRate::from_bp(750).label(), "7,50%");
    }

    #[test]
    fn tax_is_half_up() {
        assert_eq!(
            TaxRate::TVA_19.tax_on(Money::from_dzd(100)),
            Money::from_dzd(19)
        );
        assert_eq!(TaxRate::TVA_0.tax_on(Money::from_dzd(100)), Money::ZERO);
    }
}
