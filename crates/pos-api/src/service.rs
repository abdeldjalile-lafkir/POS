//! `Api` method implementations grouped by concern.

use pos_domain::{
    PaymentMethod, Product, Role, SaleStatus, SaleWithItems, StockReason, StoreInfo, User,
};
use pos_printing::ReceiptDoc;
use pos_store::products::NewProduct;
use pos_store::sales::PaymentInfo;
use pos_store::settings as sk;

use crate::dto::*;
use crate::{Api, ApiError, Result};

fn parse_role(s: &str) -> Role {
    if s.eq_ignore_ascii_case("admin") {
        Role::Admin
    } else {
        Role::Cashier
    }
}

fn parse_payment(s: &str) -> PaymentMethod {
    match s.to_ascii_lowercase().as_str() {
        "card" => PaymentMethod::Card,
        "other" => PaymentMethod::Other,
        _ => PaymentMethod::Cash,
    }
}

impl Api {
    // ---- auth -------------------------------------------------------------

    pub fn has_users(&self) -> Result<bool> {
        Ok(self.db.has_users()?)
    }

    pub fn bootstrap_admin(&mut self, username: &str, pin: &str) -> Result<User> {
        if self.db.has_users()? {
            return Err(ApiError::Message("users already exist".into()));
        }
        let id = self.db.create_user(username, pin, Role::Admin)?;
        let user = self
            .db
            .get_user(id)?
            .ok_or_else(|| ApiError::Message("user vanished".into()))?;
        self.session = Some(user.clone());
        Ok(user)
    }

    pub fn login(&mut self, username: &str, pin: &str) -> Result<User> {
        match self.db.authenticate(username, pin)? {
            Some(user) => {
                self.session = Some(user.clone());
                Ok(user)
            }
            None => Err(ApiError::Message("invalid credentials".into())),
        }
    }

    pub fn logout(&mut self) {
        self.session = None;
    }

    /// The list of account names shown on the login screen (no secrets, no auth).
    pub fn login_users(&self) -> Result<Vec<User>> {
        Ok(self.db.list_users()?)
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        self.admin()?;
        Ok(self.db.list_users()?)
    }

    pub fn create_user(&self, username: &str, pin: &str, role: &str) -> Result<i64> {
        self.admin()?;
        Ok(self.db.create_user(username, pin, parse_role(role))?)
    }

    pub fn set_user_active(&self, id: i64, active: bool) -> Result<()> {
        self.admin()?;
        self.db.set_user_active(id, active)?;
        Ok(())
    }

    pub fn reset_pin(&self, id: i64, pin: &str) -> Result<()> {
        self.admin()?;
        if self
            .session
            .as_ref()
            .map(|u| u.id == id)
            .unwrap_or(false)
        {
            // allowed: changing your own PIN
        }
        self.db.set_user_pin(id, pin)?;
        Ok(())
    }

    // ---- categories & products -------------------------------------------

    pub fn list_categories(&self) -> Result<Vec<pos_domain::Category>> {
        Ok(self.db.list_categories()?)
    }

    pub fn create_category(&self, name: &str, name_ar: &str) -> Result<i64> {
        self.admin()?;
        Ok(self.db.create_category(name, name_ar)?)
    }

    pub fn list_products(&self, include_inactive: bool) -> Result<Vec<Product>> {
        Ok(self.db.list_products(include_inactive)?)
    }

    pub fn search_products(&self, query: &str, limit: i64) -> Result<Vec<Product>> {
        Ok(self.db.search_products(query, limit)?)
    }

    pub fn get_product(&self, id: i64) -> Result<Option<Product>> {
        Ok(self.db.get_product(id)?)
    }

    pub fn find_by_barcode(&self, barcode: &str) -> Result<Option<Product>> {
        Ok(self.db.find_by_barcode(barcode)?)
    }

    pub fn save_product(&self, input: &ProductInput) -> Result<i64> {
        // Any signed-in user may create a product from the register (e.g. when a
        // scanned item is not in the catalogue). Editing stays admin-only.
        self.user()?;
        if input.id.is_some() {
            self.admin()?;
        }
        let barcode = input
            .barcode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match input.id {
            Some(id) => {
                let mut p = self
                    .db
                    .get_product(id)?
                    .ok_or_else(|| ApiError::Message(format!("product {id} not found")))?;
                p.sku = input.sku.clone();
                p.barcode = barcode.map(str::to_string);
                p.name = input.name.clone();
                p.name_ar = input.name_ar.clone();
                p.category_id = input.category_id;
                p.price = pos_domain::Money::from_centimes(input.price);
                p.tax_rate = pos_domain::TaxRate::from_bp(input.tax_rate_bp);
                p.unit = input.unit.clone();
                p.is_active = input.is_active;
                self.db.update_product(&p)?;
                Ok(id)
            }
            None => Ok(self.db.create_product(&NewProduct {
                sku: input.sku.clone(),
                barcode: barcode.map(str::to_string),
                name: input.name.clone(),
                name_ar: input.name_ar.clone(),
                category_id: input.category_id,
                price: pos_domain::Money::from_centimes(input.price),
                tax_rate: pos_domain::TaxRate::from_bp(input.tax_rate_bp),
                unit: input.unit.clone(),
            })?),
        }
    }

    pub fn set_product_active(&self, id: i64, active: bool) -> Result<()> {
        self.admin()?;
        self.db.set_product_active(id, active)?;
        Ok(())
    }

    // ---- inventory --------------------------------------------------------

    pub fn stock_overview(&self) -> Result<Vec<pos_store::inventory::StockRow>> {
        Ok(self.db.stock_overview()?)
    }

    pub fn low_stock(&self, threshold: i64) -> Result<Vec<pos_store::inventory::StockRow>> {
        Ok(self.db.low_stock(threshold)?)
    }

    pub fn stock_for(&self, product_id: i64) -> Result<i64> {
        Ok(self.db.stock_for(product_id)?)
    }

    pub fn adjust_stock(&mut self, input: &AdjustStockInput) -> Result<i64> {
        self.admin()?;
        let note = input
            .note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let user = self.session.as_ref().map(|u| u.id);
        let level = self.db.adjust_stock(
            input.product_id,
            input.delta,
            StockReason::parse(&input.reason),
            note,
            user,
        )?;
        Ok(level)
    }

    pub fn movements_for(&self, product_id: i64) -> Result<Vec<pos_domain::StockMovement>> {
        Ok(self.db.movements_for(product_id)?)
    }

    // ---- checkout ---------------------------------------------------------

    pub fn create_sale(&mut self, cart: &CartInput, payment: &PaymentInput) -> Result<SaleResult> {
        let user = self.user()?.id;
        let built = self.build_cart(cart);
        let sale = self.db.create_sale(
            &built,
            user,
            PaymentInfo {
                method: parse_payment(&payment.method),
                tendered: pos_domain::Money::from_centimes(payment.tendered),
            },
        )?;
        self.to_result(sale)
    }

    pub fn get_sale(&self, id: i64) -> Result<Option<SaleWithItems>> {
        Ok(self.db.get_sale(id)?)
    }

    pub fn last_sale(&self) -> Result<Option<SaleWithItems>> {
        Ok(self.db.last_sale()?)
    }

    pub fn refund_sale(&mut self, sale_id: i64) -> Result<SaleResult> {
        let user = self.admin()?.id;
        let sale = self.db.refund_sale(sale_id, user)?;
        self.to_result(sale)
    }

    pub fn sales_between(&self, from: i64, to: i64) -> Result<Vec<pos_domain::Sale>> {
        Ok(self.db.sales_between(from, to)?)
    }

    fn to_result(&self, sale: SaleWithItems) -> Result<SaleResult> {
        let store = self.db.store_info()?;
        let doc = ReceiptDoc::from_sale(&sale, &store, self.paper_width());
        let receipt_text = pos_printing::render_text(&doc);
        Ok(SaleResult {
            sale: sale.sale,
            items: sale.items,
            receipt_text,
        })
    }

    // ---- reports ----------------------------------------------------------

    pub fn reports(&self, from: i64, to: i64) -> Result<ReportsBundle> {
        Ok(ReportsBundle {
            from,
            to,
            z: self.db.z_report(from, to)?,
            top: self.db.top_products(from, to, 10)?,
            daily: self.db.daily_summary(from, to)?,
        })
    }

    // ---- settings ---------------------------------------------------------

    pub fn settings(&self) -> Result<AppSettings> {
        Ok(AppSettings {
            store: self.db.store_info()?,
            printer: PrinterSettings {
                transport: self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_TRANSPORT, "preview")?,
                host: self.db.get_setting_or(sk::KEY_PRINTER_HOST, "192.168.0.50")?,
                port: self.db.get_setting_or(sk::KEY_PRINTER_PORT, "9100")?,
                device: self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_DEVICE, "/dev/usb/lp0")?,
                paper_width_mm: self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_PAPER_WIDTH, "80")?
                    .parse()
                    .unwrap_or(80),
                auto_cut: self.db.get_setting_or(sk::KEY_PRINTER_AUTO_CUT, "1")? != "0",
                drawer_kick: self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_DRAWER_KICK, "1")?
                    != "0",
                scan_max_gap_ms: self
                    .db
                    .get_setting_or(sk::KEY_SCAN_MAX_GAP_MS, "30")?
                    .parse()
                    .unwrap_or(30),
            },
            language: self.db.get_setting_or(sk::KEY_LANGUAGE, "en")?,
        })
    }

    pub fn save_store_info(&self, info: &StoreInfo) -> Result<()> {
        self.admin()?;
        self.db.save_store_info(info)?;
        Ok(())
    }

    pub fn save_printer(&self, p: &PrinterSettings) -> Result<()> {
        self.admin()?;
        let pairs: [(&str, String); 8] = [
            (sk::KEY_PRINTER_TRANSPORT, p.transport.clone()),
            (sk::KEY_PRINTER_HOST, p.host.clone()),
            (sk::KEY_PRINTER_PORT, p.port.clone()),
            (sk::KEY_PRINTER_DEVICE, p.device.clone()),
            (sk::KEY_PRINTER_PAPER_WIDTH, p.paper_width_mm.to_string()),
            (
                sk::KEY_PRINTER_AUTO_CUT,
                if p.auto_cut { "1" } else { "0" }.to_string(),
            ),
            (
                sk::KEY_PRINTER_DRAWER_KICK,
                if p.drawer_kick { "1" } else { "0" }.to_string(),
            ),
            (sk::KEY_SCAN_MAX_GAP_MS, p.scan_max_gap_ms.to_string()),
        ];
        for (k, v) in pairs {
            self.db.set_setting(k, &v)?;
        }
        Ok(())
    }

    pub fn set_language(&self, lang: &str) -> Result<()> {
        self.db.set_setting(sk::KEY_LANGUAGE, lang)?;
        Ok(())
    }

    pub fn backup(&self) -> Result<String> {
        self.admin()?;
        let path = self
            .data_dir
            .join(format!("backup-{}.db", pos_domain::now_unix()));
        self.db.backup_to(&path)?;
        Ok(path.to_string_lossy().into_owned())
    }

    // ---- printing ---------------------------------------------------------

    pub fn receipt_text_for(&self, sale_id: i64) -> Result<String> {
        let sale = self
            .db
            .get_sale(sale_id)?
            .ok_or_else(|| ApiError::Message(format!("sale {sale_id} not found")))?;
        let store = self.db.store_info()?;
        let doc = ReceiptDoc::from_sale(&sale, &store, self.paper_width());
        Ok(pos_printing::render_text(&doc))
    }

    pub fn print_receipt(&self, sale_id: i64) -> Result<()> {
        let sale = self
            .db
            .get_sale(sale_id)?
            .ok_or_else(|| ApiError::Message(format!("sale {sale_id} not found")))?;
        if sale.sale.status == SaleStatus::Voided {
            return Err(ApiError::Message("sale is voided".into()));
        }
        let store = self.db.store_info()?;
        let doc = ReceiptDoc::from_sale(&sale, &store, self.paper_width());
        self.printer_config().print(&doc)?;
        Ok(())
    }

    pub fn print_test(&self) -> Result<()> {
        let store = self.db.store_info()?;
        self.printer_config().print_test(&store.name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{CartInput, CartLineInput, PaymentInput, ProductInput};

    fn api() -> Api {
        let db = pos_store::Db::open_in_memory().unwrap();
        let mut api = Api::new(db, std::env::temp_dir());
        api.bootstrap_admin("admin", "1234").unwrap();
        api
    }

    fn add_product(api: &Api, name: &str, barcode: Option<&str>, price: i64, bp: u32) -> i64 {
        api.save_product(&ProductInput {
            id: None,
            sku: String::new(),
            barcode: barcode.map(str::to_string),
            name: name.into(),
            name_ar: String::new(),
            category_id: None,
            price,
            tax_rate_bp: bp,
            unit: "pcs".into(),
            is_active: true,
        })
        .unwrap()
    }

    #[test]
    fn bootstrap_then_login() {
        let mut api = api();
        assert!(api.has_users().unwrap());
        // Bootstrap is refused once users exist.
        assert!(api.bootstrap_admin("x", "1234").is_err());

        api.logout();
        assert!(api.login("admin", "wrong").is_err());
        let u = api.login("admin", "1234").unwrap();
        assert_eq!(u.role, Role::Admin);
    }

    #[test]
    fn checkout_builds_receipt_and_updates_stock() {
        let mut api = api();
        let milk = add_product(&api, "Milk", Some("6130000000019"), 12000, 900);
        let coffee = add_product(&api, "Coffee", None, 45000, 1900);
        api.adjust_stock(&AdjustStockInput {
            product_id: milk,
            delta: 10,
            reason: "purchase".into(),
            note: None,
        })
        .unwrap();
        api.adjust_stock(&AdjustStockInput {
            product_id: coffee,
            delta: 10,
            reason: "purchase".into(),
            note: None,
        })
        .unwrap();

        // barcode lookup
        assert_eq!(
            api.find_by_barcode("6130000000019").unwrap().unwrap().id,
            milk
        );

        let cart = CartInput {
            lines: vec![
                CartLineInput {
                    product_id: Some(milk),
                    name: "Milk".into(),
                    unit_price: 12000,
                    tax_rate_bp: 900,
                    qty: 2,
                    discount: None,
                },
                CartLineInput {
                    product_id: Some(coffee),
                    name: "Coffee".into(),
                    unit_price: 45000,
                    tax_rate_bp: 1900,
                    qty: 1,
                    discount: Some(DiscountInput {
                        kind: "percent".into(),
                        value: 10.0,
                    }),
                },
            ],
            sale_discount: None,
        };
        let res = api
            .create_sale(
                &cart,
                &PaymentInput {
                    method: "cash".into(),
                    tendered: 100_000,
                },
            )
            .unwrap();

        assert!(res.receipt_text.contains("TOTAL"));
        assert!(res.receipt_text.contains("Milk"));
        assert_eq!(api.stock_for(milk).unwrap(), 8);
        assert_eq!(api.stock_for(coffee).unwrap(), 9);
        assert!(res.sale.receipt_no.ends_with("-0001"));

        // refund
        let r = api.refund_sale(res.sale.id).unwrap();
        assert_eq!(r.sale.status, SaleStatus::Refunded);
        assert_eq!(api.stock_for(milk).unwrap(), 10);
    }

    #[test]
    fn reports_bundle() {
        let mut api = api();
        let p = add_product(&api, "Bread", None, 1500, 0);
        api.create_sale(
            &CartInput {
                lines: vec![CartLineInput {
                    product_id: Some(p),
                    name: "Bread".into(),
                    unit_price: 1500,
                    tax_rate_bp: 0,
                    qty: 3,
                    discount: None,
                }],
                sale_discount: None,
            },
            &PaymentInput {
                method: "cash".into(),
                tendered: 5000,
            },
        )
        .unwrap();

        let now = pos_domain::now_unix();
        let b = api.reports(now - 60, now + 60).unwrap();
        assert_eq!(b.z.count, 1);
        assert_eq!(b.top.len(), 1);
        assert!(!b.daily.is_empty());
    }

    #[test]
    fn non_admin_is_forbidden_from_admin_ops() {
        let mut api = api();
        api.create_user("sara", "1234", "cashier").unwrap();
        api.logout();
        api.login("sara", "1234").unwrap();

        // Admin-only operations are refused...
        assert!(api.list_users().is_err());
        assert!(api.backup().is_err());

        // ...but a cashier may create a product (quick-add at the register)...
        let id = api
            .save_product(&ProductInput {
                id: None,
                sku: String::new(),
                barcode: Some("999".into()),
                name: "Loose item".into(),
                name_ar: String::new(),
                category_id: None,
                price: 5000,
                tax_rate_bp: 0,
                unit: "pcs".into(),
                is_active: true,
            })
            .unwrap();
        assert!(id > 0);

        // ...while editing an existing product stays admin-only.
        let edit = api.save_product(&ProductInput {
            id: Some(id),
            sku: String::new(),
            barcode: Some("999".into()),
            name: "Changed".into(),
            name_ar: String::new(),
            category_id: None,
            price: 6000,
            tax_rate_bp: 0,
            unit: "pcs".into(),
            is_active: true,
        });
        assert!(edit.is_err());
    }
}
