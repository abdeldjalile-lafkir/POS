//! `pos-domain` — pure, dependency-light business logic for the POS.
//!
//! Contains exact money arithmetic, Algerian TVA handling, the cart/total engine
//! and the shared entity types. No I/O, no UI, no database.

pub mod cart;
pub mod entities;
pub mod money;
pub mod tax;

pub use cart::{Cart, CartLine, Discount, TaxLine, Totals};
pub use entities::{
    Category, PaymentMethod, Product, Role, Sale, SaleItem, SaleStatus, SaleWithItems,
    StockMovement, StockReason, StoreInfo, User,
};
pub use money::Money;
pub use tax::TaxRate;

/// Current Unix time in seconds (UTC).
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
