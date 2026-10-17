//! Product and category persistence.

use pos_domain::{Category, Money, Product, TaxRate};
use rusqlite::{params, OptionalExtension, Row};

use crate::error::{Result, StoreError};
use crate::Db;

/// Fields required to create a product.
#[derive(Debug, Clone)]
pub struct NewProduct {
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub name_ar: String,
    pub category_id: Option<i64>,
    pub price: Money,
    pub tax_rate: TaxRate,
    pub unit: String,
}

impl Default for NewProduct {
    fn default() -> Self {
        Self {
            sku: String::new(),
            barcode: None,
            name: String::new(),
            name_ar: String::new(),
            category_id: None,
            price: Money::ZERO,
            tax_rate: TaxRate::TVA_19,
            unit: "pcs".to_string(),
        }
    }
}

pub(crate) fn map_product(row: &Row<'_>) -> rusqlite::Result<Product> {
    let bp: i64 = row.get("tax_rate_bp")?;
    let active: i64 = row.get("is_active")?;
    Ok(Product {
        id: row.get("id")?,
        sku: row.get("sku")?,
        barcode: row.get("barcode")?,
        name: row.get("name")?,
        name_ar: row.get("name_ar")?,
        category_id: row.get("category_id")?,
        price: Money::from_centimes(row.get("price_centimes")?),
        tax_rate: TaxRate::from_bp(bp as u32),
        unit: row.get("unit")?,
        is_active: active != 0,
        image_path: row.get("image_path")?,
    })
}

impl Db {
    pub fn list_products(&self, include_inactive: bool) -> Result<Vec<Product>> {
        let sql = if include_inactive {
            "SELECT * FROM products ORDER BY name COLLATE NOCASE"
        } else {
            "SELECT * FROM products WHERE is_active = 1 ORDER BY name COLLATE NOCASE"
        };
        let mut stmt = self.conn().prepare(sql)?;
        let rows = stmt.query_map([], map_product)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Full-text-ish search across name, Arabic name, SKU and exact barcode.
    pub fn search_products(&self, query: &str, limit: i64) -> Result<Vec<Product>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let like = format!("%{q}%");
        let mut stmt = self.conn().prepare(
            "SELECT * FROM products
             WHERE is_active = 1 AND (
                 name LIKE ?1 OR name_ar LIKE ?1 OR sku LIKE ?1 OR barcode = ?2
             )
             ORDER BY (barcode = ?2) DESC, name COLLATE NOCASE
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![like, q, limit], map_product)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_product(&self, id: i64) -> Result<Option<Product>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT * FROM products WHERE id = ?1",
                params![id],
                map_product,
            )
            .optional()?)
    }

    pub fn find_by_barcode(&self, barcode: &str) -> Result<Option<Product>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT * FROM products WHERE barcode = ?1",
                params![barcode.trim()],
                map_product,
            )
            .optional()?)
    }

    pub fn create_product(&self, p: &NewProduct) -> Result<i64> {
        if p.name.trim().is_empty() {
            return Err(StoreError::Invalid("product name is required".into()));
        }
        let barcode = p
            .barcode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        self.conn().execute(
            "INSERT INTO products
                (sku, barcode, name, name_ar, category_id, price_centimes, tax_rate_bp, unit, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
            params![
                p.sku.trim(),
                barcode,
                p.name.trim(),
                p.name_ar.trim(),
                p.category_id,
                p.price.centimes(),
                p.tax_rate.bp() as i64,
                p.unit.trim(),
            ],
        )?;
        let id = self.conn().last_insert_rowid();
        self.conn().execute(
            "INSERT OR IGNORE INTO stock_levels (product_id, quantity) VALUES (?1, 0)",
            params![id],
        )?;
        Ok(id)
    }

    pub fn update_product(&self, p: &Product) -> Result<()> {
        if p.name.trim().is_empty() {
            return Err(StoreError::Invalid("product name is required".into()));
        }
        let barcode = p
            .barcode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let n = self.conn().execute(
            "UPDATE products SET
                sku = ?2, barcode = ?3, name = ?4, name_ar = ?5, category_id = ?6,
                price_centimes = ?7, tax_rate_bp = ?8, unit = ?9, is_active = ?10
             WHERE id = ?1",
            params![
                p.id,
                p.sku.trim(),
                barcode,
                p.name.trim(),
                p.name_ar.trim(),
                p.category_id,
                p.price.centimes(),
                p.tax_rate.bp() as i64,
                p.unit.trim(),
                p.is_active as i64,
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("product {}", p.id)));
        }
        Ok(())
    }

    pub fn set_product_active(&self, id: i64, active: bool) -> Result<()> {
        self.conn().execute(
            "UPDATE products SET is_active = ?2 WHERE id = ?1",
            params![id, active as i64],
        )?;
        Ok(())
    }

    /// Bulk insert/update products in a **single transaction** using prepared
    /// statements — the fast path for large catalogue imports (tens of
    /// thousands of rows). Products are matched by barcode (updated) or created.
    ///
    /// Returns the resulting ids (aligned with `items`) plus `(added, updated)`.
    pub fn upsert_products_bulk(
        &mut self,
        items: &[NewProduct],
    ) -> Result<(Vec<i64>, i64, i64)> {
        let tx = self.conn_mut().transaction()?;
        let mut ids = Vec::with_capacity(items.len());
        let mut added = 0i64;
        let mut updated = 0i64;
        {
            let mut find = tx.prepare("SELECT id FROM products WHERE barcode = ?1")?;
            let mut ins = tx.prepare(
                "INSERT INTO products
                    (sku, barcode, name, name_ar, category_id, price_centimes, tax_rate_bp, unit, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
            )?;
            let mut upd = tx.prepare(
                "UPDATE products SET
                    sku = ?2, name = ?3, name_ar = ?4, category_id = ?5,
                    price_centimes = ?6, tax_rate_bp = ?7, unit = ?8
                 WHERE id = ?1",
            )?;
            let mut lvl = tx.prepare(
                "INSERT OR IGNORE INTO stock_levels (product_id, quantity) VALUES (?1, 0)",
            )?;

            for p in items {
                let barcode = p
                    .barcode
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                let existing: Option<i64> = match barcode {
                    Some(b) => find.query_row(params![b], |r| r.get(0)).optional()?,
                    None => None,
                };
                let id = match existing {
                    Some(id) => {
                        upd.execute(params![
                            id,
                            p.sku.trim(),
                            p.name.trim(),
                            p.name_ar.trim(),
                            p.category_id,
                            p.price.centimes(),
                            p.tax_rate.bp() as i64,
                            p.unit.trim(),
                        ])?;
                        updated += 1;
                        id
                    }
                    None => {
                        ins.execute(params![
                            p.sku.trim(),
                            barcode,
                            p.name.trim(),
                            p.name_ar.trim(),
                            p.category_id,
                            p.price.centimes(),
                            p.tax_rate.bp() as i64,
                            p.unit.trim(),
                        ])?;
                        let id = tx.last_insert_rowid();
                        lvl.execute(params![id])?;
                        added += 1;
                        id
                    }
                };
                ids.push(id);
            }
        }
        tx.commit()?;
        Ok((ids, added, updated))
    }

    // ---- categories ----

    pub fn list_categories(&self) -> Result<Vec<Category>> {
        let mut stmt = self
            .conn()
            .prepare("SELECT * FROM categories ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get("id")?,
                name: row.get("name")?,
                name_ar: row.get("name_ar")?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn create_category(&self, name: &str, name_ar: &str) -> Result<i64> {
        if name.trim().is_empty() {
            return Err(StoreError::Invalid("category name is required".into()));
        }
        self.conn().execute(
            "INSERT INTO categories (name, name_ar) VALUES (?1, ?2)",
            params![name.trim(), name_ar.trim()],
        )?;
        Ok(self.conn().last_insert_rowid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str, barcode: Option<&str>, price_dzd: i64) -> NewProduct {
        NewProduct {
            name: name.into(),
            barcode: barcode.map(String::from),
            price: Money::from_dzd(price_dzd),
            ..Default::default()
        }
    }

    #[test]
    fn create_list_search_and_barcode() {
        let db = Db::open_in_memory().unwrap();
        let id = db
            .create_product(&sample("Milk 1L", Some("6130000000019"), 120))
            .unwrap();
        db.create_product(&sample("Bread", None, 15)).unwrap();

        assert_eq!(db.list_products(false).unwrap().len(), 2);
        let p = db.get_product(id).unwrap().unwrap();
        assert_eq!(p.price, Money::from_dzd(120));
        assert_eq!(p.tax_rate, TaxRate::TVA_19);

        assert_eq!(db.find_by_barcode("6130000000019").unwrap().unwrap().id, id);
        assert!(db.find_by_barcode("nope").unwrap().is_none());

        let hits = db.search_products("mil", 10).unwrap();
        assert_eq!(hits.len(), 1);
        let hits = db.search_products("6130000000019", 10).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn unique_barcode_is_enforced() {
        let db = Db::open_in_memory().unwrap();
        db.create_product(&sample("A", Some("111"), 1)).unwrap();
        assert!(db.create_product(&sample("B", Some("111"), 1)).is_err());
    }

    #[test]
    fn updates_and_deactivation() {
        let db = Db::open_in_memory().unwrap();
        let id = db.create_product(&sample("A", None, 10)).unwrap();
        let mut p = db.get_product(id).unwrap().unwrap();
        p.name = "A2".into();
        p.price = Money::from_dzd(20);
        db.update_product(&p).unwrap();
        assert_eq!(db.get_product(id).unwrap().unwrap().name, "A2");
        db.set_product_active(id, false).unwrap();
        assert_eq!(db.list_products(false).unwrap().len(), 0);
        assert_eq!(db.list_products(true).unwrap().len(), 1);
    }
}
