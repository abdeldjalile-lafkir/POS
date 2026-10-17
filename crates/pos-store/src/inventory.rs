//! Stock levels (fast cache) and the movement ledger (audit trail).
//!
//! Levels and movements are always written together inside one transaction.

use pos_domain::{Money, Product, StockMovement, StockReason};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;

use crate::error::{Result, StoreError};
use crate::products::map_product;
use crate::Db;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockRow {
    pub product: Product,
    pub quantity: i64,
}

fn map_stock_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StockRow> {
    // The product columns come first, then quantity.
    let product = {
        // Re-read the product columns by name.
        let bp: i64 = row.get("tax_rate_bp")?;
        let active: i64 = row.get("is_active")?;
        Product {
            id: row.get("id")?,
            sku: row.get("sku")?,
            barcode: row.get("barcode")?,
            name: row.get("name")?,
            name_ar: row.get("name_ar")?,
            category_id: row.get("category_id")?,
            price: Money::from_centimes(row.get("price_centimes")?),
            tax_rate: pos_domain::TaxRate::from_bp(bp as u32),
            unit: row.get("unit")?,
            is_active: active != 0,
            image_path: row.get("image_path")?,
        }
    };
    Ok(StockRow {
        product,
        quantity: row.get("quantity")?,
    })
}

/// Apply a stock delta inside an existing transaction, writing both the ledger
/// entry and the cached level. Used by the sale transaction.
pub(crate) fn apply_delta_tx(
    tx: &Transaction<'_>,
    product_id: i64,
    delta: i64,
    reason: StockReason,
    ref_sale_id: Option<i64>,
    note: Option<&str>,
    user_id: Option<i64>,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO stock_levels (product_id, quantity) VALUES (?1, ?2)
         ON CONFLICT(product_id) DO UPDATE SET quantity = quantity + ?2",
        params![product_id, delta],
    )?;
    tx.execute(
        "INSERT INTO stock_movements
            (product_id, delta, reason, ref_sale_id, note, user_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            product_id,
            delta,
            reason.as_str(),
            ref_sale_id,
            note,
            user_id,
            pos_domain::now_unix(),
        ],
    )?;
    Ok(())
}

/// Read a product's cached level from any connection (used inside transactions).
pub(crate) fn level_on(conn: &Connection, product_id: i64) -> rusqlite::Result<i64> {
    Ok(conn
        .query_row(
            "SELECT quantity FROM stock_levels WHERE product_id = ?1",
            params![product_id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

impl Db {
    pub fn stock_for(&self, product_id: i64) -> Result<i64> {
        Ok(level_on(self.conn(), product_id)?)
    }

    pub fn stock_overview(&self) -> Result<Vec<StockRow>> {
        let mut stmt = self.conn().prepare(
            "SELECT p.*, COALESCE(s.quantity, 0) AS quantity
             FROM products p
             LEFT JOIN stock_levels s ON s.product_id = p.id
             WHERE p.is_active = 1
             ORDER BY p.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], map_stock_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn low_stock(&self, threshold: i64) -> Result<Vec<StockRow>> {
        let mut stmt = self.conn().prepare(
            "SELECT p.*, COALESCE(s.quantity, 0) AS quantity
             FROM products p
             LEFT JOIN stock_levels s ON s.product_id = p.id
             WHERE p.is_active = 1 AND COALESCE(s.quantity, 0) <= ?1
             ORDER BY quantity ASC, p.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map(params![threshold], map_stock_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Manually adjust stock (purchase, correction, etc.). Returns the new level.
    pub fn adjust_stock(
        &mut self,
        product_id: i64,
        delta: i64,
        reason: StockReason,
        note: Option<&str>,
        user_id: Option<i64>,
    ) -> Result<i64> {
        if self.get_product(product_id)?.is_none() {
            return Err(StoreError::NotFound(format!("product {product_id}")));
        }
        let tx = self.conn_mut().transaction()?;
        apply_delta_tx(&tx, product_id, delta, reason, None, note, user_id)?;
        let new_level = level_on(&tx, product_id)?;
        tx.commit()?;
        Ok(new_level)
    }

    pub fn movements_for(&self, product_id: i64) -> Result<Vec<StockMovement>> {
        let mut stmt = self.conn().prepare(
            "SELECT * FROM stock_movements WHERE product_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT 200",
        )?;
        let rows = stmt.query_map(params![product_id], |row| {
            let reason: String = row.get("reason")?;
            Ok(StockMovement {
                id: row.get("id")?,
                product_id: row.get("product_id")?,
                delta: row.get("delta")?,
                reason: StockReason::parse(&reason),
                ref_sale_id: row.get("ref_sale_id")?,
                note: row.get("note")?,
                user_id: row.get("user_id")?,
                created_at: row.get("created_at")?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

// Keep the mapper available for potential reuse.
#[allow(dead_code)]
fn _use_map_product(row: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    map_product(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::products::NewProduct;

    fn mk(db: &Db, name: &str) -> i64 {
        db.create_product(&NewProduct {
            name: name.into(),
            price: Money::from_dzd(10),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn adjust_updates_level_and_ledger() {
        let mut db = Db::open_in_memory().unwrap();
        let id = mk(&db, "Widget");
        assert_eq!(db.stock_for(id).unwrap(), 0);

        let lvl = db
            .adjust_stock(id, 100, StockReason::Purchase, Some("initial"), None)
            .unwrap();
        assert_eq!(lvl, 100);
        let lvl = db
            .adjust_stock(id, -5, StockReason::Adjustment, Some("shrinkage"), None)
            .unwrap();
        assert_eq!(lvl, 95);

        let moves = db.movements_for(id).unwrap();
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].delta, -5);
        assert_eq!(moves[0].reason, StockReason::Adjustment);
    }

    #[test]
    fn low_stock_filter() {
        let mut db = Db::open_in_memory().unwrap();
        let a = mk(&db, "A");
        let b = mk(&db, "B");
        db.adjust_stock(a, 3, StockReason::Purchase, None, None)
            .unwrap();
        db.adjust_stock(b, 50, StockReason::Purchase, None, None)
            .unwrap();
        let low = db.low_stock(5).unwrap();
        assert_eq!(low.len(), 1);
        assert_eq!(low[0].product.name, "A");
        assert_eq!(low[0].quantity, 3);
    }
}
