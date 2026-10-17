//! The cart and the sale-total engine.
//!
//! Tax model (per the plan): product prices are **tax-exclusive (HT)**; TVA is
//! added at checkout and shown broken down by rate on the receipt.
//!
//! Rounding policy (also per the plan): tax is computed **per line** and rounded
//! **half-up**, then summed into the per-rate breakdown shown on the receipt.
//! Per-line rounding keeps each stored sale line internally consistent with the
//! receipt total. A sale-level discount is spread across lines proportionally to
//! their net value using largest-remainder, so the tax bases stay exact.

use crate::money::Money;
use crate::tax::TaxRate;
use serde::{Deserialize, Serialize};

/// A discount, either a percentage (basis points) or a fixed amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Discount {
    #[default]
    None,
    /// Percentage in basis points (1000 = 10%).
    PercentBp(u32),
    /// Fixed amount off.
    Amount(Money),
}

impl Discount {
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Discount::None)
    }

    /// The discount amount applied to `base`, clamped to `[0, base]`.
    pub fn compute(&self, base: Money) -> Money {
        match self {
            Discount::None => Money::ZERO,
            Discount::PercentBp(bp) => base.percent_bp(*bp).clamp_to(Money::ZERO, base),
            Discount::Amount(m) => (*m).clamp_to(Money::ZERO, base),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Discount::None => String::new(),
            Discount::PercentBp(bp) => format!("{}%", *bp as f64 / 100.0),
            Discount::Amount(m) => m.format_amount(),
        }
    }
}

/// A line in the cart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartLine {
    /// `None` for loose / non-catalogue items.
    pub product_id: Option<i64>,
    pub name: String,
    pub unit_price: Money,
    pub tax_rate: TaxRate,
    pub qty: i64,
    pub discount: Discount,
}

impl CartLine {
    pub fn new(
        product_id: Option<i64>,
        name: impl Into<String>,
        unit_price: Money,
        tax_rate: TaxRate,
    ) -> Self {
        Self {
            product_id,
            name: name.into(),
            unit_price,
            tax_rate,
            qty: 1,
            discount: Discount::None,
        }
    }

    /// Gross (before line discount) = unit price * qty.
    pub fn gross(&self) -> Money {
        self.unit_price.mul_qty(self.qty)
    }

    pub fn line_discount(&self) -> Money {
        self.discount.compute(self.gross())
    }

    /// Net of line discount (still tax-exclusive).
    pub fn net(&self) -> Money {
        self.gross() - self.line_discount()
    }

    /// Tax on this line, if it were taxed alone.
    pub fn tax(&self) -> Money {
        self.tax_rate.tax_on(self.net())
    }

    pub fn total(&self) -> Money {
        self.net() + self.tax()
    }
}

/// The working cart.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cart {
    pub lines: Vec<CartLine>,
    pub sale_discount: Discount,
}

impl Cart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.sale_discount = Discount::None;
    }

    /// Add a product or increment its quantity if already present.
    pub fn add_or_increment(&mut self, line: CartLine) {
        if let Some(existing) = self
            .lines
            .iter_mut()
            .find(|l| l.product_id.is_some() && l.product_id == line.product_id)
        {
            existing.qty += line.qty;
        } else {
            self.lines.push(line);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.lines.len() {
            self.lines.remove(index);
        }
    }

    pub fn set_qty(&mut self, index: usize, qty: i64) {
        if let Some(line) = self.lines.get_mut(index) {
            if qty <= 0 {
                line.qty = 0;
            } else {
                line.qty = qty;
            }
        }
        self.lines.retain(|l| l.qty > 0);
    }

    pub fn totals(&self) -> Totals {
        Totals::compute(&self.lines, self.sale_discount)
    }
}

/// Tax broken down for one rate, for the receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxLine {
    pub rate: TaxRate,
    /// Taxable base (HT) for this rate.
    pub base: Money,
    pub tax: Money,
}

/// Computed sale totals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    /// Sum of unit_price*qty before any discount.
    pub gross: Money,
    /// Sum of line-level discounts.
    pub line_discounts: Money,
    /// Net (HT) after line discounts, before the sale discount.
    pub subtotal: Money,
    /// Sale-level discount actually applied.
    pub sale_discount: Money,
    /// Total discounts (line + sale).
    pub discount_total: Money,
    pub tax_lines: Vec<TaxLine>,
    pub tax_total: Money,
    /// Grand total (TTC).
    pub total: Money,
}

impl Totals {
    /// The taxable base (HT) of each line after the sale discount has been spread
    /// across lines. Provided so the store can persist exact per-line amounts.
    pub fn line_bases(lines: &[CartLine], sale_discount: Discount) -> Vec<Money> {
        let nets: Vec<Money> = lines.iter().map(CartLine::net).collect();
        let subtotal: Money = nets.iter().copied().sum();
        let allocated = allocate_proportionally(sale_discount.compute(subtotal), &nets);
        nets.iter()
            .zip(allocated.iter())
            .map(|(net, a)| *net - *a)
            .collect()
    }

    pub fn compute(lines: &[CartLine], sale_discount: Discount) -> Totals {
        let mut gross = Money::ZERO;
        let mut line_discounts = Money::ZERO;
        let mut nets: Vec<Money> = Vec::with_capacity(lines.len());

        for line in lines {
            let g = line.gross();
            let d = line.discount.compute(g);
            gross += g;
            line_discounts += d;
            nets.push(g - d);
        }

        let subtotal: Money = nets.iter().copied().sum();
        let sale_discount_amount = sale_discount.compute(subtotal);

        // Spread the sale discount across lines proportionally to their net value.
        let allocated = allocate_proportionally(sale_discount_amount, &nets);

        // Per-line tax (rounded half-up per line) aggregated by rate. Summing the
        // per-line taxes keeps the persisted sale lines and the receipt total exact.
        let mut tax_lines: Vec<TaxLine> = Vec::new();
        let mut tax_total = Money::ZERO;
        for (line, alloc) in lines.iter().zip(allocated.iter()) {
            let base = line.net() - *alloc;
            let tax = line.tax_rate.tax_on(base);
            tax_total += tax;
            match tax_lines.iter_mut().find(|g| g.rate == line.tax_rate) {
                Some(g) => {
                    g.base += base;
                    g.tax += tax;
                }
                None => tax_lines.push(TaxLine {
                    rate: line.tax_rate,
                    base,
                    tax,
                }),
            }
        }
        tax_lines.sort_by_key(|g| std::cmp::Reverse(g.rate.bp()));

        let net_after_sale = subtotal - sale_discount_amount;
        Totals {
            gross,
            line_discounts,
            subtotal,
            sale_discount: sale_discount_amount,
            discount_total: line_discounts + sale_discount_amount,
            tax_lines,
            tax_total,
            total: net_after_sale + tax_total,
        }
    }
}

/// Split `total` across `weights` proportionally, using largest-remainder so the
/// parts sum exactly to `total`. All weights must be non-negative.
fn allocate_proportionally(total: Money, weights: &[Money]) -> Vec<Money> {
    let n = weights.len();
    let mut out = vec![Money::ZERO; n];
    let t = total.centimes();
    if n == 0 || t <= 0 {
        return out;
    }
    let sum_w: i64 = weights.iter().map(|m| m.centimes()).sum();
    if sum_w <= 0 {
        return out;
    }

    let mut remainders: Vec<(usize, i64)> = Vec::with_capacity(n);
    let mut assigned: i64 = 0;
    for (i, w) in weights.iter().enumerate() {
        let num = t.saturating_mul(w.centimes());
        let base = num / sum_w;
        let rem = num % sum_w;
        out[i] = Money::from_centimes(base);
        assigned += base;
        remainders.push((i, rem));
    }

    let mut left = t - assigned;
    remainders.sort_by_key(|(_, r)| std::cmp::Reverse(*r));
    let mut k = 0;
    while left > 0 && !remainders.is_empty() {
        let (idx, _) = remainders[k % remainders.len()];
        out[idx] += Money::from_centimes(1);
        left -= 1;
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

    fn line(price_dzd: i64, rate: TaxRate, qty: i64) -> CartLine {
        let mut l = CartLine::new(None, "x", Money::from_dzd(price_dzd), rate);
        l.qty = qty;
        l
    }

    #[test]
    fn simple_sale_adds_tax() {
        // 1 x 100,00 DZD @ 19% -> HT 100, TVA 19, TTC 119
        let lines = vec![line(100, TaxRate::TVA_19, 1)];
        let t = Totals::compute(&lines, Discount::None);
        assert_eq!(t.subtotal, Money::from_dzd(100));
        assert_eq!(t.tax_total, Money::from_dzd(19));
        assert_eq!(t.total, Money::from_dzd(119));
        assert_eq!(t.tax_lines.len(), 1);
    }

    #[test]
    fn mixed_rates_break_down_per_rate() {
        let lines = vec![
            line(100, TaxRate::TVA_19, 1), // base 100, tax 19
            line(50, TaxRate::TVA_9, 2),   // base 100, tax 9
            line(30, TaxRate::TVA_0, 1),   // base 30,  tax 0
        ];
        let t = Totals::compute(&lines, Discount::None);
        assert_eq!(t.subtotal, Money::from_dzd(230));
        assert_eq!(t.tax_total, Money::from_dzd(28));
        assert_eq!(t.total, Money::from_dzd(258));
        // sorted by rate desc: 19, 9, 0
        assert_eq!(t.tax_lines[0].rate, TaxRate::TVA_19);
        assert_eq!(t.tax_lines[0].tax, Money::from_dzd(19));
        assert_eq!(t.tax_lines[1].rate, TaxRate::TVA_9);
        assert_eq!(t.tax_lines[1].tax, Money::from_dzd(9));
        assert_eq!(t.tax_lines[2].rate, TaxRate::TVA_0);
        assert_eq!(t.tax_lines[2].tax, Money::ZERO);
    }

    #[test]
    fn line_discount_reduces_base_before_tax() {
        let mut l = line(100, TaxRate::TVA_19, 1);
        l.discount = Discount::PercentBp(1000); // 10%
        let t = Totals::compute(&[l], Discount::None);
        assert_eq!(t.line_discounts, Money::from_dzd(10));
        assert_eq!(t.subtotal, Money::from_dzd(90));
        assert_eq!(t.tax_total, Money::from_centimes(1710)); // 19% of 90.00 = 17.10
        assert_eq!(t.total, Money::from_centimes(10710));
    }

    #[test]
    fn sale_discount_spreads_and_sums_exactly() {
        // Two lines with awkward split: total discount 1 centime across 1:1 weights.
        let lines = vec![
            CartLine::new(None, "a", Money::from_centimes(1), TaxRate::TVA_0),
            CartLine::new(None, "b", Money::from_centimes(1), TaxRate::TVA_0),
        ];
        let t = Totals::compute(&lines, Discount::Amount(Money::from_centimes(1)));
        assert_eq!(t.sale_discount, Money::from_centimes(1));
        assert_eq!(t.total, Money::from_centimes(1)); // 2 - 1
    }

    #[test]
    fn sale_discount_percent_of_subtotal() {
        let lines = vec![line(100, TaxRate::TVA_19, 1), line(100, TaxRate::TVA_19, 1)];
        let t = Totals::compute(&lines, Discount::PercentBp(1000)); // 10% of 200 = 20
        assert_eq!(t.sale_discount, Money::from_dzd(20));
        assert_eq!(t.subtotal, Money::from_dzd(200));
        assert_eq!(t.tax_total, Money::from_centimes(3420)); // 19% of 180.00 = 34.20
        assert_eq!(t.total, Money::from_centimes(21420));
    }

    #[test]
    fn fixed_discount_is_clamped_to_subtotal() {
        let lines = vec![line(10, TaxRate::TVA_19, 1)];
        let t = Totals::compute(&lines, Discount::Amount(Money::from_dzd(999)));
        assert_eq!(t.sale_discount, Money::from_dzd(10));
        assert_eq!(t.total, Money::ZERO);
    }

    #[test]
    fn allocation_preserves_total() {
        let weights = vec![
            Money::from_centimes(333),
            Money::from_centimes(333),
            Money::from_centimes(334),
        ];
        let parts = allocate_proportionally(Money::from_centimes(1000), &weights);
        let sum: i64 = parts.iter().map(|m| m.centimes()).sum();
        assert_eq!(sum, 1000);
    }

    #[test]
    fn cart_add_or_increment() {
        let mut cart = Cart::new();
        cart.add_or_increment(CartLine::new(
            Some(1),
            "Milk",
            Money::from_dzd(120),
            TaxRate::TVA_9,
        ));
        cart.add_or_increment(CartLine::new(
            Some(1),
            "Milk",
            Money::from_dzd(120),
            TaxRate::TVA_9,
        ));
        assert_eq!(cart.lines.len(), 1);
        assert_eq!(cart.lines[0].qty, 2);
    }
}
