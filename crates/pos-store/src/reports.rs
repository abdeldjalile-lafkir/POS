//! Reporting: sales summaries, TVA by rate, top products and the Z-report.

use pos_domain::{Money, TaxRate};
use rusqlite::{params, Connection};

use crate::error::Result;
use crate::Db;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateAgg {
    pub rate: TaxRate,
    pub base: Money,
    pub tax: Money,
}

#[derive(Debug, Clone)]
pub struct DaySummary {
    pub day: String,
    pub count: i64,
    pub gross: Money,
    pub discount: Money,
    pub tax_total: Money,
    pub total: Money,
    pub cash: Money,
    pub card: Money,
}

#[derive(Debug, Clone)]
pub struct TopProduct {
    pub name: String,
    pub qty: i64,
    pub total: Money,
}

#[derive(Debug, Clone, Default)]
pub struct ZReport {
    pub from: i64,
    pub to: i64,
    pub count: i64,
    pub gross: Money,
    pub discount: Money,
    pub tax_total: Money,
    pub total: Money,
    pub cash_total: Money,
    pub card_total: Money,
    pub other_total: Money,
    pub refunds_total: Money,
    pub by_rate: Vec<RateAgg>,
}

impl ZReport {
    /// Net cash expected in the drawer (cash sales minus cash refunds).
    pub fn cash_expected(&self) -> Money {
        self.cash_total - self.refunds_total
    }
}

fn by_rate(conn: &Connection, from: i64, to: i64) -> rusqlite::Result<Vec<RateAgg>> {
    let mut stmt = conn.prepare(
        "SELECT si.tax_rate_bp AS bp,
                COALESCE(SUM(si.line_subtotal_c), 0) AS base,
                COALESCE(SUM(si.line_tax_c), 0) AS tax
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         WHERE s.status = 'completed' AND s.created_at >= ?1 AND s.created_at < ?2
         GROUP BY si.tax_rate_bp
         ORDER BY bp DESC",
    )?;
    let rows = stmt.query_map(params![from, to], |row| {
        let bp: i64 = row.get("bp")?;
        Ok(RateAgg {
            rate: TaxRate::from_bp(bp as u32),
            base: Money::from_centimes(row.get("base")?),
            tax: Money::from_centimes(row.get("tax")?),
        })
    })?;
    rows.collect()
}

impl Db {
    pub fn z_report(&self, from: i64, to: i64) -> Result<ZReport> {
        let mut r = ZReport {
            from,
            to,
            ..Default::default()
        };

        self.conn().query_row(
            "SELECT COUNT(*) AS count,
                    COALESCE(SUM(subtotal_c + discount_c), 0) AS gross,
                    COALESCE(SUM(discount_c), 0) AS discount,
                    COALESCE(SUM(tax_total_c), 0) AS tax_total,
                    COALESCE(SUM(total_c), 0) AS total,
                    COALESCE(SUM(CASE WHEN payment_method = 'Cash' THEN total_c END), 0) AS cash,
                    COALESCE(SUM(CASE WHEN payment_method = 'Card' THEN total_c END), 0) AS card,
                    COALESCE(SUM(CASE WHEN payment_method = 'Other' THEN total_c END), 0) AS other
             FROM sales
             WHERE status = 'completed' AND created_at >= ?1 AND created_at < ?2",
            params![from, to],
            |row| {
                r.count = row.get("count")?;
                r.gross = Money::from_centimes(row.get("gross")?);
                r.discount = Money::from_centimes(row.get("discount")?);
                r.tax_total = Money::from_centimes(row.get("tax_total")?);
                r.total = Money::from_centimes(row.get("total")?);
                r.cash_total = Money::from_centimes(row.get("cash")?);
                r.card_total = Money::from_centimes(row.get("card")?);
                r.other_total = Money::from_centimes(row.get("other")?);
                Ok(())
            },
        )?;

        r.refunds_total = self.conn().query_row(
            "SELECT COALESCE(SUM(total_c), 0) FROM refunds WHERE created_at >= ?1 AND created_at < ?2",
            params![from, to],
            |row| Ok(Money::from_centimes(row.get(0)?)),
        )?;

        r.by_rate = by_rate(self.conn(), from, to)?;
        Ok(r)
    }

    pub fn top_products(&self, from: i64, to: i64, limit: i64) -> Result<Vec<TopProduct>> {
        let mut stmt = self.conn().prepare(
            "SELECT si.name_snapshot AS name,
                    SUM(si.qty) AS qty,
                    SUM(si.line_total_c) AS total
             FROM sale_items si
             JOIN sales s ON s.id = si.sale_id
             WHERE s.status = 'completed' AND s.created_at >= ?1 AND s.created_at < ?2
             GROUP BY si.name_snapshot
             ORDER BY qty DESC, total DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![from, to, limit], |row| {
            Ok(TopProduct {
                name: row.get("name")?,
                qty: row.get("qty")?,
                total: Money::from_centimes(row.get("total")?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Sales aggregated by local calendar day.
    pub fn daily_summary(&self, from: i64, to: i64) -> Result<Vec<DaySummary>> {
        let mut stmt = self.conn().prepare(
            "SELECT date(created_at, 'unixepoch', 'localtime') AS day,
                    COUNT(*) AS count,
                    COALESCE(SUM(subtotal_c + discount_c), 0) AS gross,
                    COALESCE(SUM(discount_c), 0) AS discount,
                    COALESCE(SUM(tax_total_c), 0) AS tax_total,
                    COALESCE(SUM(total_c), 0) AS total,
                    COALESCE(SUM(CASE WHEN payment_method = 'Cash' THEN total_c END), 0) AS cash,
                    COALESCE(SUM(CASE WHEN payment_method = 'Card' THEN total_c END), 0) AS card
             FROM sales
             WHERE status = 'completed' AND created_at >= ?1 AND created_at < ?2
             GROUP BY day
             ORDER BY day DESC",
        )?;
        let rows = stmt.query_map(params![from, to], |row| {
            Ok(DaySummary {
                day: row.get("day")?,
                count: row.get("count")?,
                gross: Money::from_centimes(row.get("gross")?),
                discount: Money::from_centimes(row.get("discount")?),
                tax_total: Money::from_centimes(row.get("tax_total")?),
                total: Money::from_centimes(row.get("total")?),
                cash: Money::from_centimes(row.get("cash")?),
                card: Money::from_centimes(row.get("card")?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::products::NewProduct;
    use crate::sales::PaymentInfo;
    use pos_domain::{Cart, CartLine, PaymentMethod, Role, TaxRate};

    #[allow(clippy::too_many_arguments)]
    fn sell(
        db: &mut Db,
        cashier: i64,
        pid: i64,
        name: &str,
        price_dzd: i64,
        rate: TaxRate,
        qty: i64,
        cash: bool,
    ) {
        let mut cart = Cart::new();
        let mut line = CartLine::new(Some(pid), name, Money::from_dzd(price_dzd), rate);
        line.qty = qty;
        cart.lines.push(line);
        db.create_sale(
            &cart,
            cashier,
            PaymentInfo {
                method: if cash {
                    PaymentMethod::Cash
                } else {
                    PaymentMethod::Card
                },
                tendered: if cash {
                    Money::from_dzd(price_dzd * qty + 1000)
                } else {
                    Money::ZERO
                },
            },
        )
        .unwrap();
    }

    #[test]
    fn z_report_totals_and_rate_breakdown() {
        let mut db = Db::open_in_memory().unwrap();
        let cashier = db.create_user("admin", "1234", Role::Admin).unwrap();
        let p19 = db
            .create_product(&NewProduct {
                name: "A".into(),
                price: Money::from_dzd(100),
                tax_rate: TaxRate::TVA_19,
                ..Default::default()
            })
            .unwrap();
        let p9 = db
            .create_product(&NewProduct {
                name: "B".into(),
                price: Money::from_dzd(50),
                tax_rate: TaxRate::TVA_9,
                ..Default::default()
            })
            .unwrap();

        sell(&mut db, cashier, p19, "A", 100, TaxRate::TVA_19, 1, true); // 100 + 19 = 119
        sell(&mut db, cashier, p9, "B", 50, TaxRate::TVA_9, 2, false); //  100 + 9 = 109

        let now = pos_domain::now_unix();
        let r = db.z_report(now - 3600, now + 3600).unwrap();
        assert_eq!(r.count, 2);
        assert_eq!(r.total, Money::from_dzd(228));
        assert_eq!(r.tax_total, Money::from_dzd(28));
        assert_eq!(r.cash_total, Money::from_dzd(119));
        assert_eq!(r.card_total, Money::from_dzd(109));
        assert_eq!(r.by_rate.len(), 2);
        assert_eq!(r.by_rate[0].rate, TaxRate::TVA_19);
        assert_eq!(r.by_rate[0].tax, Money::from_dzd(19));
        assert_eq!(r.by_rate[1].rate, TaxRate::TVA_9);
        assert_eq!(r.by_rate[1].tax, Money::from_dzd(9));
        assert_eq!(r.cash_expected(), Money::from_dzd(119));
    }

    #[test]
    fn refunds_reduce_cash_expected() {
        let mut db = Db::open_in_memory().unwrap();
        let cashier = db.create_user("admin", "1234", Role::Admin).unwrap();
        let pid = db
            .create_product(&NewProduct {
                name: "A".into(),
                price: Money::from_dzd(100),
                ..Default::default()
            })
            .unwrap();
        sell(&mut db, cashier, pid, "A", 100, TaxRate::TVA_19, 1, true);
        let sale = db.last_sale().unwrap().unwrap();
        db.refund_sale(sale.sale.id, cashier).unwrap();

        let now = pos_domain::now_unix();
        let r = db.z_report(now - 3600, now + 3600).unwrap();
        // Completed sales excludes the refunded sale.
        assert_eq!(r.count, 0);
        assert_eq!(r.refunds_total, Money::from_dzd(119));
    }

    #[test]
    fn top_products_ranks_by_quantity() {
        let mut db = Db::open_in_memory().unwrap();
        let cashier = db.create_user("admin", "1234", Role::Admin).unwrap();
        let a = db
            .create_product(&NewProduct {
                name: "A".into(),
                price: Money::from_dzd(10),
                ..Default::default()
            })
            .unwrap();
        let b = db
            .create_product(&NewProduct {
                name: "B".into(),
                price: Money::from_dzd(10),
                ..Default::default()
            })
            .unwrap();
        sell(&mut db, cashier, a, "A", 10, TaxRate::TVA_19, 3, true);
        sell(&mut db, cashier, b, "B", 10, TaxRate::TVA_19, 1, true);
        let now = pos_domain::now_unix();
        let top = db.top_products(now - 3600, now + 3600, 10).unwrap();
        assert_eq!(top[0].name, "A");
        assert_eq!(top[0].qty, 3);
    }
}
