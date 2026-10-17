//! Structured receipt model, built from a stored sale.

use std::collections::BTreeMap;

use chrono::{Local, TimeZone};
use pos_domain::{Money, SaleWithItems, StoreInfo, TaxRate};

/// Thermal paper width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperWidth {
    Mm58,
    Mm80,
}

impl PaperWidth {
    /// Character columns for the standard font.
    pub const fn cols(self) -> usize {
        match self {
            PaperWidth::Mm58 => 32,
            PaperWidth::Mm80 => 48,
        }
    }

    pub const fn mm(self) -> u32 {
        match self {
            PaperWidth::Mm58 => 58,
            PaperWidth::Mm80 => 80,
        }
    }

    pub fn from_mm(mm: u32) -> PaperWidth {
        if mm >= 80 {
            PaperWidth::Mm80
        } else {
            PaperWidth::Mm58
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReceiptLine {
    pub name: String,
    pub qty: i64,
    pub unit_price: Money,
    pub total: Money,
}

#[derive(Debug, Clone)]
pub struct ReceiptTaxLine {
    pub label: String,
    pub base: Money,
    pub tax: Money,
}

/// A fully-rendered receipt, ready to print.
#[derive(Debug, Clone)]
pub struct ReceiptDoc {
    pub store: StoreInfo,
    pub receipt_no: String,
    pub datetime: String,
    pub cashier: String,
    pub title: &'static str,
    pub lines: Vec<ReceiptLine>,
    pub tax_lines: Vec<ReceiptTaxLine>,
    pub subtotal: Money,
    pub discount_total: Money,
    pub tax_total: Money,
    pub total: Money,
    pub payment_method: String,
    pub tendered: Money,
    pub change: Money,
    pub footer: String,
    pub paper_width: PaperWidth,
}

impl ReceiptDoc {
    /// Build a receipt from a stored sale.
    pub fn from_sale(sale: &SaleWithItems, store: &StoreInfo, paper_width: PaperWidth) -> Self {
        let items = &sale.items;

        let mut lines = Vec::with_capacity(items.len());
        let mut tax_map: BTreeMap<u32, (TaxRate, Money, Money)> = BTreeMap::new();
        for item in items {
            lines.push(ReceiptLine {
                name: item.name_snapshot.clone(),
                qty: item.qty,
                unit_price: item.unit_price,
                total: item.line_total,
            });
            let entry = tax_map.entry(item.tax_rate.bp()).or_insert((
                item.tax_rate,
                Money::ZERO,
                Money::ZERO,
            ));
            entry.1 += item.line_subtotal;
            entry.2 += item.line_tax;
        }
        // Highest rate first.
        let tax_lines = tax_map
            .into_iter()
            .rev()
            .map(|(_, (rate, base, tax))| ReceiptTaxLine {
                label: rate.label(),
                base,
                tax,
            })
            .collect();

        let title = if sale.sale.status == pos_domain::SaleStatus::Refunded {
            "REFUND"
        } else {
            "RECEIPT"
        };

        ReceiptDoc {
            store: store.clone(),
            receipt_no: sale.sale.receipt_no.clone(),
            datetime: format_local(sale.sale.created_at),
            cashier: sale.sale.cashier_name.clone(),
            title,
            lines,
            tax_lines,
            subtotal: sale.sale.subtotal,
            discount_total: sale.sale.discount_total,
            tax_total: sale.sale.tax_total,
            total: sale.sale.total,
            payment_method: sale.sale.payment_method.as_str().to_string(),
            tendered: sale.sale.tendered,
            change: sale.sale.change,
            footer: store.receipt_footer.clone(),
            paper_width,
        }
    }
}

/// Format a Unix timestamp in the machine's local time.
pub fn format_local(unix: i64) -> String {
    Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| unix.to_string())
}
