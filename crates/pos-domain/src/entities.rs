//! Shared entity types used by the store, printing and API layers.
//!
//! Serde uses `camelCase` so the JSON consumed by the web UI is idiomatic there.

use crate::money::Money;
use crate::tax::TaxRate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub name_ar: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub name_ar: String,
    pub category_id: Option<i64>,
    /// Tax-exclusive unit price, in centimes.
    pub price: Money,
    pub tax_rate: TaxRate,
    pub unit: String,
    pub is_active: bool,
    pub image_path: Option<String>,
}

impl Product {
    /// Localised display name.
    pub fn display_name(&self, arabic: bool) -> &str {
        if arabic && !self.name_ar.trim().is_empty() {
            &self.name_ar
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Cashier,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Cashier => "Cashier",
        }
    }

    pub fn parse(s: &str) -> Role {
        match s {
            "Admin" => Role::Admin,
            _ => Role::Cashier,
        }
    }

    pub const fn is_admin(self) -> bool {
        matches!(self, Role::Admin)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub is_active: bool,
    /// Unix timestamp (seconds, UTC).
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Cash,
    Card,
    Other,
}

impl PaymentMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            PaymentMethod::Cash => "Cash",
            PaymentMethod::Card => "Card",
            PaymentMethod::Other => "Other",
        }
    }

    pub fn parse(s: &str) -> PaymentMethod {
        match s {
            "Card" => PaymentMethod::Card,
            "Other" => PaymentMethod::Other,
            _ => PaymentMethod::Cash,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaleStatus {
    Completed,
    Refunded,
    Voided,
}

impl SaleStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            SaleStatus::Completed => "completed",
            SaleStatus::Refunded => "refunded",
            SaleStatus::Voided => "voided",
        }
    }

    pub fn parse(s: &str) -> SaleStatus {
        match s {
            "refunded" => SaleStatus::Refunded,
            "voided" => SaleStatus::Voided,
            _ => SaleStatus::Completed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StockReason {
    Sale,
    Purchase,
    Adjustment,
    Return,
}

impl StockReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            StockReason::Sale => "sale",
            StockReason::Purchase => "purchase",
            StockReason::Adjustment => "adjustment",
            StockReason::Return => "return",
        }
    }

    pub fn parse(s: &str) -> StockReason {
        match s {
            "purchase" => StockReason::Purchase,
            "adjustment" => StockReason::Adjustment,
            "return" => StockReason::Return,
            _ => StockReason::Sale,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: i64,
    pub product_id: i64,
    pub delta: i64,
    pub reason: StockReason,
    pub ref_sale_id: Option<i64>,
    pub note: Option<String>,
    pub user_id: Option<i64>,
    pub created_at: i64,
}

/// A persisted sale line (with snapshots so history stays correct).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleItem {
    pub id: i64,
    pub sale_id: i64,
    pub product_id: Option<i64>,
    pub name_snapshot: String,
    pub unit_price: Money,
    pub tax_rate: TaxRate,
    pub qty: i64,
    pub discount: Money,
    pub line_subtotal: Money,
    pub line_tax: Money,
    pub line_total: Money,
}

/// A persisted sale header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sale {
    pub id: i64,
    pub receipt_no: String,
    pub created_at: i64,
    pub cashier_id: i64,
    pub cashier_name: String,
    pub subtotal: Money,
    pub tax_total: Money,
    pub discount_total: Money,
    pub total: Money,
    pub payment_method: PaymentMethod,
    pub tendered: Money,
    pub change: Money,
    pub status: SaleStatus,
}

/// A sale together with its lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleWithItems {
    pub sale: Sale,
    pub items: Vec<SaleItem>,
}

/// Store legal / identity information printed on receipts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreInfo {
    pub name: String,
    pub address: String,
    pub nif: String,
    pub nis: String,
    pub rc: String,
    pub ai: String,
    pub phone: String,
    pub receipt_footer: String,
}
