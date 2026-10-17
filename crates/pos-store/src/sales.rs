//! Sales: the checkout transaction, sale retrieval, and refunds.
//!
//! `create_sale` persists the sale, its lines, the stock decrements and the stock
//! ledger entries in a **single transaction**. Receipt numbers are allocated from
//! a per-day counter inside that same transaction, so numbering is sequential and
//! gap-free.

use pos_domain::{
    Cart, Money, PaymentMethod, Sale, SaleItem, SaleStatus, SaleWithItems, StockReason, TaxRate,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::error::{Result, StoreError};
use crate::inventory::apply_delta_tx;
use crate::Db;

/// Payment details captured at checkout.
#[derive(Debug, Clone, Copy)]
pub struct PaymentInfo {
    pub method: PaymentMethod,
    pub tendered: Money,
}

fn local_day(now: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(now, 0)
        .single()
        .map(|dt| dt.format("%Y%m%d").to_string())
        .unwrap_or_else(|| "00000000".to_string())
}

fn next_receipt_no_tx(tx: &Transaction<'_>, now: i64) -> rusqlite::Result<String> {
    let day = local_day(now);
    let key = format!("receipt:{day}");
    tx.execute(
        "INSERT INTO counters (name, value) VALUES (?1, 1)
         ON CONFLICT(name) DO UPDATE SET value = value + 1",
        params![key],
    )?;
    let seq: i64 = tx.query_row(
        "SELECT value FROM counters WHERE name = ?1",
        params![key],
        |r| r.get(0),
    )?;
    Ok(format!("{day}-{seq:04}"))
}

fn map_sale(row: &rusqlite::Row<'_>) -> rusqlite::Result<Sale> {
    let method: String = row.get("payment_method")?;
    let status: String = row.get("status")?;
    Ok(Sale {
        id: row.get("id")?,
        receipt_no: row.get("receipt_no")?,
        created_at: row.get("created_at")?,
        cashier_id: row.get("cashier_id")?,
        cashier_name: row.get("cashier_name")?,
        subtotal: Money::from_centimes(row.get("subtotal_c")?),
        tax_total: Money::from_centimes(row.get("tax_total_c")?),
        discount_total: Money::from_centimes(row.get("discount_c")?),
        total: Money::from_centimes(row.get("total_c")?),
        payment_method: PaymentMethod::parse(&method),
        tendered: Money::from_centimes(row.get("tendered_c")?),
        change: Money::from_centimes(row.get("change_c")?),
        status: SaleStatus::parse(&status),
    })
}

fn map_sale_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<SaleItem> {
    let bp: i64 = row.get("tax_rate_bp")?;
    Ok(SaleItem {
        id: row.get("id")?,
        sale_id: row.get("sale_id")?,
        product_id: row.get("product_id")?,
        name_snapshot: row.get("name_snapshot")?,
        unit_price: Money::from_centimes(row.get("unit_price_c")?),
        tax_rate: TaxRate::from_bp(bp as u32),
        qty: row.get("qty")?,
        discount: Money::from_centimes(row.get("discount_c")?),
        line_subtotal: Money::from_centimes(row.get("line_subtotal_c")?),
        line_tax: Money::from_centimes(row.get("line_tax_c")?),
        line_total: Money::from_centimes(row.get("line_total_c")?),
    })
}

const SALE_SELECT: &str = "SELECT s.*, u.username AS cashier_name
     FROM sales s JOIN users u ON u.id = s.cashier_id";

impl Db {
    /// Persist a completed sale atomically. Returns the stored sale + items.
    pub fn create_sale(
        &mut self,
        cart: &Cart,
        cashier_id: i64,
        payment: PaymentInfo,
    ) -> Result<SaleWithItems> {
        if cart.is_empty() {
            return Err(StoreError::Invalid("cart is empty".into()));
        }
        let totals = cart.totals();
        let bases = pos_domain::Totals::line_bases(&cart.lines, cart.sale_discount);

        let change = if payment.method == PaymentMethod::Cash {
            (payment.tendered - totals.total).clamp_to(Money::ZERO, Money::from_centimes(i64::MAX))
        } else {
            Money::ZERO
        };
        if payment.method == PaymentMethod::Cash && payment.tendered < totals.total {
            return Err(StoreError::Invalid(
                "tendered amount is less than the total".into(),
            ));
        }

        let now = pos_domain::now_unix();
        let tx = self.conn_mut().transaction()?;

        let receipt_no = next_receipt_no_tx(&tx, now)?;

        tx.execute(
            "INSERT INTO sales
                (receipt_no, created_at, cashier_id, subtotal_c, tax_total_c, discount_c,
                 total_c, payment_method, tendered_c, change_c, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'completed')",
            params![
                receipt_no,
                now,
                cashier_id,
                totals.subtotal.centimes(),
                totals.tax_total.centimes(),
                totals.discount_total.centimes(),
                totals.total.centimes(),
                payment.method.as_str(),
                payment.tendered.centimes(),
                change.centimes(),
            ],
        )?;
        let sale_id = tx.last_insert_rowid();

        for (line, base) in cart.lines.iter().zip(bases.iter()) {
            let tax = line.tax_rate.tax_on(*base);
            let line_total = *base + tax;
            tx.execute(
                "INSERT INTO sale_items
                    (sale_id, product_id, name_snapshot, unit_price_c, tax_rate_bp, qty,
                     discount_c, line_subtotal_c, line_tax_c, line_total_c)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sale_id,
                    line.product_id,
                    line.name,
                    line.unit_price.centimes(),
                    line.tax_rate.bp() as i64,
                    line.qty,
                    line.line_discount().centimes(),
                    base.centimes(),
                    tax.centimes(),
                    line_total.centimes(),
                ],
            )?;

            if let Some(pid) = line.product_id {
                apply_delta_tx(
                    &tx,
                    pid,
                    -line.qty,
                    StockReason::Sale,
                    Some(sale_id),
                    None,
                    Some(cashier_id),
                )?;
            }
        }

        tx.commit()?;
        self.get_sale(sale_id)?
            .ok_or_else(|| StoreError::Other("sale vanished after insert".into()))
    }

    pub fn get_sale(&self, id: i64) -> Result<Option<SaleWithItems>> {
        let sale = self
            .conn()
            .query_row(
                &format!("{SALE_SELECT} WHERE s.id = ?1"),
                params![id],
                map_sale,
            )
            .optional()?;
        let Some(sale) = sale else { return Ok(None) };
        let items = self.sale_items(sale.id)?;
        Ok(Some(SaleWithItems { sale, items }))
    }

    pub fn get_sale_by_receipt(&self, receipt_no: &str) -> Result<Option<SaleWithItems>> {
        let sale = self
            .conn()
            .query_row(
                &format!("{SALE_SELECT} WHERE s.receipt_no = ?1"),
                params![receipt_no.trim()],
                map_sale,
            )
            .optional()?;
        let Some(sale) = sale else { return Ok(None) };
        let items = self.sale_items(sale.id)?;
        Ok(Some(SaleWithItems { sale, items }))
    }

    pub fn last_sale(&self) -> Result<Option<SaleWithItems>> {
        let sale = self
            .conn()
            .query_row(
                &format!("{SALE_SELECT} ORDER BY s.id DESC LIMIT 1"),
                [],
                map_sale,
            )
            .optional()?;
        let Some(sale) = sale else { return Ok(None) };
        let items = self.sale_items(sale.id)?;
        Ok(Some(SaleWithItems { sale, items }))
    }

    fn sale_items(&self, sale_id: i64) -> Result<Vec<SaleItem>> {
        let mut stmt = self
            .conn()
            .prepare("SELECT * FROM sale_items WHERE sale_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map(params![sale_id], map_sale_item)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Sales in `[from, to)` by creation time.
    pub fn sales_between(&self, from: i64, to: i64) -> Result<Vec<Sale>> {
        let mut stmt = self.conn().prepare(&format!(
            "{SALE_SELECT} WHERE s.created_at >= ?1 AND s.created_at < ?2 ORDER BY s.created_at"
        ))?;
        let rows = stmt.query_map(params![from, to], map_sale)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn sales_count(&self) -> Result<i64> {
        Ok(self
            .conn()
            .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))?)
    }

    /// Full refund: records an audit row, flips the sale status, and returns the
    /// stock. The original sale row is retained (never deleted).
    pub fn refund_sale(&mut self, sale_id: i64, user_id: i64) -> Result<SaleWithItems> {
        let existing = self
            .get_sale(sale_id)?
            .ok_or_else(|| StoreError::NotFound(format!("sale {sale_id}")))?;
        if existing.sale.status != SaleStatus::Completed {
            return Err(StoreError::Invalid("sale is not refundable".into()));
        }

        let now = pos_domain::now_unix();
        let tx = self.conn_mut().transaction()?;
        let refund_receipt = next_receipt_no_tx(&tx, now)?;

        for item in &existing.items {
            if let Some(pid) = item.product_id {
                apply_delta_tx(
                    &tx,
                    pid,
                    item.qty,
                    StockReason::Return,
                    Some(sale_id),
                    Some("refund"),
                    Some(user_id),
                )?;
            }
        }

        tx.execute(
            "UPDATE sales SET status = 'refunded' WHERE id = ?1",
            params![sale_id],
        )?;
        tx.execute(
            "INSERT INTO refunds (sale_id, receipt_no, created_at, user_id, total_c)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                sale_id,
                refund_receipt,
                now,
                user_id,
                existing.sale.total.centimes()
            ],
        )?;
        tx.commit()?;

        self.get_sale(sale_id)?
            .ok_or_else(|| StoreError::Other("sale vanished after refund".into()))
    }
}

/// Unused helper kept for symmetry with `inventory`.
#[allow(dead_code)]
fn _touch(_c: &Connection) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::products::NewProduct;
    use pos_domain::{CartLine, Discount, Role, TaxRate};

    fn setup() -> (Db, i64, i64) {
        let db = Db::open_in_memory().unwrap();
        let admin = db.create_user("admin", "1234", Role::Admin).unwrap();
        let pid = db
            .create_product(&NewProduct {
                name: "Apple".into(),
                price: Money::from_dzd(100),
                tax_rate: TaxRate::TVA_19,
                ..Default::default()
            })
            .unwrap();
        (db, admin, pid)
    }

    fn cart_with(pid: i64, qty: i64, price_dzd: i64) -> Cart {
        let mut cart = Cart::new();
        let mut line = CartLine::new(
            Some(pid),
            "Apple",
            Money::from_dzd(price_dzd),
            TaxRate::TVA_19,
        );
        line.qty = qty;
        cart.lines.push(line);
        cart
    }

    #[test]
    fn sale_is_persisted_with_stock_and_ledger() {
        let (mut db, cashier, pid) = setup();
        db.adjust_stock(pid, 10, StockReason::Purchase, None, Some(cashier))
            .unwrap();

        let sale = db
            .create_sale(
                &cart_with(pid, 2, 100),
                cashier,
                PaymentInfo {
                    method: PaymentMethod::Cash,
                    tendered: Money::from_dzd(300),
                },
            )
            .unwrap();

        assert_eq!(sale.sale.subtotal, Money::from_dzd(200));
        assert_eq!(sale.sale.tax_total, Money::from_dzd(38));
        assert_eq!(sale.sale.total, Money::from_dzd(238));
        assert_eq!(sale.sale.change, Money::from_dzd(62));
        assert_eq!(sale.items.len(), 1);
        assert_eq!(sale.items[0].line_total, Money::from_dzd(238));
        assert_eq!(db.stock_for(pid).unwrap(), 8);
        assert!(sale
            .sale
            .receipt_no
            .starts_with(&local_day(sale.sale.created_at)));
    }

    #[test]
    fn receipt_numbers_are_sequential_per_day() {
        let (mut db, cashier, pid) = setup();
        let a = db
            .create_sale(
                &cart_with(pid, 1, 100),
                cashier,
                PaymentInfo {
                    method: PaymentMethod::Card,
                    tendered: Money::ZERO,
                },
            )
            .unwrap();
        let b = db
            .create_sale(
                &cart_with(pid, 1, 100),
                cashier,
                PaymentInfo {
                    method: PaymentMethod::Card,
                    tendered: Money::ZERO,
                },
            )
            .unwrap();
        assert!(a.sale.receipt_no.ends_with("-0001"));
        assert!(b.sale.receipt_no.ends_with("-0002"));
    }

    #[test]
    fn rejects_underpayment_and_empty_cart() {
        let (mut db, cashier, pid) = setup();
        let err = db.create_sale(
            &cart_with(pid, 1, 100),
            cashier,
            PaymentInfo {
                method: PaymentMethod::Cash,
                tendered: Money::from_dzd(50),
            },
        );
        assert!(err.is_err());

        let err = db.create_sale(
            &Cart::new(),
            cashier,
            PaymentInfo {
                method: PaymentMethod::Cash,
                tendered: Money::ZERO,
            },
        );
        assert!(err.is_err());
    }

    #[test]
    fn refund_restores_stock_and_is_audited() {
        let (mut db, cashier, pid) = setup();
        db.adjust_stock(pid, 5, StockReason::Purchase, None, Some(cashier))
            .unwrap();
        let sale = db
            .create_sale(
                &cart_with(pid, 2, 100),
                cashier,
                PaymentInfo {
                    method: PaymentMethod::Cash,
                    tendered: Money::from_dzd(300),
                },
            )
            .unwrap();
        assert_eq!(db.stock_for(pid).unwrap(), 3);

        let refunded = db.refund_sale(sale.sale.id, cashier).unwrap();
        assert_eq!(refunded.sale.status, SaleStatus::Refunded);
        assert_eq!(db.stock_for(pid).unwrap(), 5);
        // Refunding twice is refused.
        assert!(db.refund_sale(sale.sale.id, cashier).is_err());
    }

    #[test]
    fn sale_discount_is_persisted() {
        let (mut db, cashier, pid) = setup();
        let mut cart = cart_with(pid, 1, 100);
        cart.sale_discount = Discount::PercentBp(1000); // 10%
        let sale = db
            .create_sale(
                &cart,
                cashier,
                PaymentInfo {
                    method: PaymentMethod::Card,
                    tendered: Money::ZERO,
                },
            )
            .unwrap();
        assert_eq!(sale.sale.discount_total, Money::from_dzd(10));
        assert_eq!(sale.sale.subtotal, Money::from_dzd(100));
        assert_eq!(sale.sale.total, Money::from_centimes(10710));
    }
}
