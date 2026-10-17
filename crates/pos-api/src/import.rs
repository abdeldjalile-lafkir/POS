//! Catalogue import from CSV, used both for the bundled seed data and for
//! user-initiated imports. CSV parsing is RFC4180-lite (quoted fields, doubled
//! quotes, CRLF tolerant) so it accepts exports from Excel / LibreOffice.

use pos_domain::{Money, StockReason, TaxRate};
use pos_store::products::NewProduct;
use serde::Serialize;

use crate::{Api, ApiError, Result};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub products_added: i64,
    pub products_updated: i64,
    pub categories_added: i64,
    pub stock_adjusted: i64,
    pub rows_skipped: i64,
}

impl ImportSummary {
    pub fn total(&self) -> i64 {
        self.products_added + self.products_updated
    }
}

/// Parse CSV text into rows of fields.
fn parse_csv_rows(input: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                _ => field.push(c),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    // Trim whitespace and drop blank rows.
    for r in &mut rows {
        for f in r.iter_mut() {
            *f = f.trim().to_string();
        }
    }
    rows.retain(|r| r.iter().any(|f| !f.is_empty()));
    rows
}

fn normalize_header(h: &str) -> String {
    h.trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .replace("é", "e")
}

/// Percent (e.g. `9`, `19%`) to basis points.
fn parse_vat_bp(s: &str) -> u32 {
    let cleaned = s.trim().trim_end_matches('%').replace(',', ".");
    cleaned
        .parse::<f64>()
        .map(|v| (v * 100.0).round().max(0.0) as u32)
        .unwrap_or(0)
}

impl Api {
    /// Import catalogue rows. Requires admin.
    pub fn import_catalogue_csv(&mut self, csv: &str) -> Result<ImportSummary> {
        self.admin()?;
        self.import_catalogue_inner(csv)
    }

    /// Apply the bundled seed data, but only when the catalogue is empty.
    /// Safe to call on every startup.
    pub fn seed_if_empty(&mut self, csv: &str) -> Result<ImportSummary> {
        if !self.db.list_products(true)?.is_empty() {
            return Ok(ImportSummary::default());
        }
        self.import_catalogue_inner(csv)
    }

    fn import_catalogue_inner(&mut self, csv: &str) -> Result<ImportSummary> {
        let rows = parse_csv_rows(csv);
        if rows.is_empty() {
            return Err(ApiError::Message("empty CSV".into()));
        }
        let header: Vec<String> = rows[0].iter().map(|h| normalize_header(h)).collect();
        let find = |name: &str| header.iter().position(|h| h == name);
        let idx_name = find("name").ok_or_else(|| {
            ApiError::Message("CSV is missing a `name` column".into())
        })?;
        let idx_price = find("price");
        let idx_barcode = find("barcode");
        let idx_name_ar = find("name_ar").or_else(|| find("name_ar_"));
        let idx_cat = find("category");
        let idx_cat_ar = find("category_ar");
        let idx_vat = find("vat").or_else(|| find("tva")).or_else(|| find("tax"));
        let idx_unit = find("unit");
        let idx_stock = find("stock").or_else(|| find("quantity"));
        let idx_sku = find("sku");

        let get = |row: &[String], i: Option<usize>| -> String {
            i.and_then(|i| row.get(i)).cloned().unwrap_or_default()
        };

        let mut summary = ImportSummary::default();
        let user = self.session.as_ref().map(|u| u.id);

        for row in rows.iter().skip(1) {
            let name = row.get(idx_name).cloned().unwrap_or_default();
            if name.trim().is_empty() {
                summary.rows_skipped += 1;
                continue;
            }
            let price = idx_price
                .and_then(|i| row.get(i))
                .and_then(|s| Money::parse(s))
                .unwrap_or(Money::ZERO);
            let vat_bp = idx_vat
                .and_then(|i| row.get(i))
                .map(|s| parse_vat_bp(s))
                .unwrap_or(0);
            let barcode = {
                let b = get(row, idx_barcode);
                if b.is_empty() {
                    None
                } else {
                    Some(b)
                }
            };
            let name_ar = get(row, idx_name_ar);
            let unit = {
                let u = get(row, idx_unit);
                if u.is_empty() {
                    "pcs".to_string()
                } else {
                    u
                }
            };

            // Resolve / create the category.
            let mut category_id = None;
            let cat_name = get(row, idx_cat);
            if !cat_name.is_empty() {
                let existing = self
                    .db
                    .list_categories()?
                    .into_iter()
                    .find(|c| c.name.eq_ignore_ascii_case(&cat_name));
                category_id = match existing {
                    Some(c) => Some(c.id),
                    None => {
                        let id = self
                            .db
                            .create_category(&cat_name, &get(row, idx_cat_ar))?;
                        summary.categories_added += 1;
                        Some(id)
                    }
                };
            }

            // Update by barcode, else create.
            let existing = match &barcode {
                Some(b) => self.db.find_by_barcode(b)?,
                None => None,
            };
            let product_id = if let Some(mut p) = existing {
                p.name = name;
                if !name_ar.is_empty() {
                    p.name_ar = name_ar;
                }
                p.price = price;
                p.tax_rate = TaxRate::from_bp(vat_bp);
                p.unit = unit;
                if category_id.is_some() {
                    p.category_id = category_id;
                }
                let sku = get(row, idx_sku);
                if !sku.is_empty() {
                    p.sku = sku;
                }
                self.db.update_product(&p)?;
                summary.products_updated += 1;
                p.id
            } else {
                let sku = get(row, idx_sku);
                let id = self.db.create_product(&NewProduct {
                    sku,
                    barcode: barcode.clone(),
                    name,
                    name_ar,
                    category_id,
                    price,
                    tax_rate: TaxRate::from_bp(vat_bp),
                    unit,
                })?;
                summary.products_added += 1;
                id
            };

            // Optional absolute stock count.
            if let Some(si) = idx_stock {
                if let Some(raw) = row.get(si) {
                    if let Ok(target) = raw.trim().parse::<i64>() {
                        let current = self.db.stock_for(product_id)?;
                        let delta = target - current;
                        if delta != 0 {
                            self.db.adjust_stock(
                                product_id,
                                delta,
                                StockReason::Purchase,
                                Some("catalogue import"),
                                user,
                            )?;
                            summary.stock_adjusted += 1;
                        }
                    }
                }
            }
        }

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = "barcode,name,name_ar,category,category_ar,price,vat,unit,stock\n\
6130000000019,Milk 1L,حليب 1 لتر,Boissons,مشروبات,120.00,9,pcs,42\n\
\"Bread, large\",Bread,خبز,Bakery,,15.00,0,pcs,30\n\
6130000000040,Coffee 250g,قهوة 250 غ,Épicerie,بقالة,450.00,19,pcs,7\n";

    fn api() -> Api {
        let db = pos_store::Db::open_in_memory().unwrap();
        let mut api = Api::new(db, std::env::temp_dir());
        api.bootstrap_admin("admin", "1234").unwrap();
        api
    }

    #[test]
    fn csv_parses_quotes_and_commas() {
        let rows = parse_csv_rows(SEED);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1][1], "Milk 1L");
        assert_eq!(rows[2][0], "Bread, large");
        assert_eq!(rows[2][1], "Bread");
    }

    #[test]
    fn imports_products_categories_and_stock() {
        let mut api = api();
        let s = api.import_catalogue_csv(SEED).unwrap();
        assert_eq!(s.products_added, 3);
        assert_eq!(s.categories_added, 3);
        assert_eq!(s.stock_adjusted, 3);

        let products = api.list_products(false).unwrap();
        assert_eq!(products.len(), 3);
        let milk = products.iter().find(|p| p.name == "Milk 1L").unwrap();
        assert_eq!(milk.price, Money::from_dzd(120));
        assert_eq!(milk.tax_rate, TaxRate::TVA_9);
        assert_eq!(api.stock_for(milk.id).unwrap(), 42);

        // The quoted name kept its comma and created no extra category.
        let bread = api.find_by_barcode("Bread, large").unwrap().unwrap();
        assert_eq!(bread.tax_rate, TaxRate::TVA_0);
        assert_eq!(api.stock_for(bread.id).unwrap(), 30);
    }

    #[test]
    fn import_is_idempotent_by_barcode() {
        let mut api = api();
        api.import_catalogue_csv(SEED).unwrap();
        // Re-import with a changed price for the same barcode.
        let updated = SEED.replace("120.00", "135.50");
        let s = api.import_catalogue_csv(&updated).unwrap();
        assert_eq!(s.products_added, 0);
        assert_eq!(s.products_updated, 3);
        assert_eq!(api.list_products(false).unwrap().len(), 3);
        let milk = api
            .find_by_barcode("6130000000019")
            .unwrap()
            .unwrap();
        assert_eq!(milk.price, Money::from_centimes(13550));
    }

    #[test]
    fn seed_only_runs_when_empty() {
        let mut api = api();
        assert_eq!(api.seed_if_empty(SEED).unwrap().products_added, 3);
        // Second call is a no-op.
        assert_eq!(api.seed_if_empty(SEED).unwrap().total(), 0);
        assert_eq!(api.list_products(false).unwrap().len(), 3);
    }
}
