//! Data-transfer objects for the JSON API consumed by the web UI.

use pos_domain::{Discount, Money};
use serde::{Deserialize, Serialize};

/// A discount from the client: `kind` is `none` | `percent` | `amount`.
/// For `percent`, `value` is a percentage (e.g. `10` = 10%). For `amount`,
/// `value` is in centimes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountInput {
    pub kind: String,
    #[serde(default)]
    pub value: f64,
}

impl DiscountInput {
    pub fn to_discount(&self) -> Option<Discount> {
        match self.kind.as_str() {
            "percent" => {
                let bp = (self.value * 100.0).round() as u32;
                if bp == 0 {
                    None
                } else {
                    Some(Discount::PercentBp(bp))
                }
            }
            "amount" => {
                let c = self.value.round() as i64;
                if c <= 0 {
                    None
                } else {
                    Some(Discount::Amount(Money::from_centimes(c)))
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartLineInput {
    pub product_id: Option<i64>,
    pub name: String,
    /// Unit price in centimes.
    pub unit_price: i64,
    pub tax_rate_bp: u32,
    pub qty: i64,
    #[serde(default)]
    pub discount: Option<DiscountInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartInput {
    pub lines: Vec<CartLineInput>,
    #[serde(default)]
    pub sale_discount: Option<DiscountInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub id: Option<i64>,
    #[serde(default)]
    pub sku: String,
    #[serde(default)]
    pub barcode: Option<String>,
    pub name: String,
    #[serde(default)]
    pub name_ar: String,
    #[serde(default)]
    pub category_id: Option<i64>,
    /// Tax-exclusive price in centimes.
    pub price: i64,
    pub tax_rate_bp: u32,
    #[serde(default = "default_unit")]
    pub unit: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_unit() -> String {
    "pcs".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustStockInput {
    pub product_id: i64,
    pub delta: i64,
    /// purchase | adjustment | return | sale
    pub reason: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    /// cash | card | other
    pub method: String,
    /// Tendered amount in centimes (cash only).
    #[serde(default)]
    pub tendered: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleResult {
    pub sale: pos_domain::Sale,
    pub items: Vec<pos_domain::SaleItem>,
    /// Plain-text receipt, ready for on-screen preview.
    pub receipt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterSettings {
    pub transport: String,
    pub host: String,
    pub port: String,
    pub device: String,
    pub paper_width_mm: u32,
    pub auto_cut: bool,
    pub drawer_kick: bool,
    pub scan_max_gap_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub store: pos_domain::StoreInfo,
    pub printer: PrinterSettings,
    pub language: String,
}

/// Aggregate for the Reports screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportsBundle {
    pub from: i64,
    pub to: i64,
    pub z: pos_store::reports::ZReport,
    pub top: Vec<pos_store::reports::TopProduct>,
    pub daily: Vec<pos_store::reports::DaySummary>,
}
