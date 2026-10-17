//! `pos-ui` — the egui application: login, register, products, inventory,
//! reports, settings and users.
//!
//! egui 0.37's text stack (harfrust + fontique) performs Arabic shaping and BiDi
//! automatically, and system fonts supply the Arabic glyphs.

pub mod inventory;
pub mod login;
pub mod products;
pub mod register;
pub mod reports;
pub mod settings;
pub mod users;
pub mod util;

use std::path::PathBuf;

use eframe::egui;
use pos_domain::{Cart, StockReason, StoreInfo, User};
use pos_i18n::{t, Key, Lang};
use pos_printing::{PaperWidth, PrinterConfig, Transport};
use pos_store::inventory::StockRow;
use pos_store::reports::{DaySummary, TopProduct, ZReport};
use pos_store::{Db, StoreError};

use util::{ScanBuffer, Toast};

/// Which main screen is shown (after login).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Register,
    Products,
    Inventory,
    Reports,
    Settings,
    Users,
}

impl Screen {
    fn key(self) -> Key {
        match self {
            Screen::Register => Key::NavRegister,
            Screen::Products => Key::NavProducts,
            Screen::Inventory => Key::NavInventory,
            Screen::Reports => Key::NavReports,
            Screen::Settings => Key::NavSettings,
            Screen::Users => Key::NavUsers,
        }
    }

    fn admin_only(self) -> bool {
        !matches!(self, Screen::Register)
    }
}

/// Reporting period selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Today,
    Week,
    Month,
}

/// Product add/edit form.
#[derive(Debug, Clone)]
pub struct ProductForm {
    pub id: Option<i64>,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub name_ar: String,
    pub price: String,
    pub tax_index: usize,
    pub unit: String,
    pub category_id: Option<i64>,
    pub is_active: bool,
}

/// User add form.
#[derive(Debug, Clone)]
pub struct UserForm {
    pub username: String,
    pub pin: String,
    pub role_admin: bool,
}

/// Checkout payment dialog state.
pub struct PayState {
    pub method: usize, // 0 cash, 1 card, 2 other
    pub tendered: String,
}

/// The application.
pub struct PosApp {
    pub db: Db,
    pub data_dir: PathBuf,

    pub lang: Lang,
    pub session: Option<User>,
    pub has_users: bool,
    pub screen: Screen,
    pub toast: Option<Toast>,

    // login
    pub login_username: String,
    pub login_pin: String,
    pub login_error: Option<String>,
    pub login_attempts: u32,

    // first run
    pub fr_username: String,
    pub fr_pin: String,
    pub fr_pin2: String,
    pub fr_error: Option<String>,

    // register
    pub cart: Cart,
    pub search: String,
    pub results: Vec<pos_domain::Product>,
    pub scan: ScanBuffer,
    pub scan_gap_ms: u64,
    pub simulate: String,
    pub barcode_msg: Option<(String, bool)>,
    pub pay: Option<PayState>,
    pub receipt_view: Option<(pos_printing::ReceiptDoc, String)>,
    pub create_for_barcode: Option<String>,

    // products
    pub products: Vec<pos_domain::Product>,
    pub product_filter: String,
    pub show_inactive: bool,
    pub product_form: Option<ProductForm>,

    // inventory
    pub stock: Vec<StockRow>,
    pub low_only: bool,
    pub threshold: i64,
    pub inv_selected: Option<i64>,
    pub movements: Vec<pos_domain::StockMovement>,
    pub adjust_delta: String,
    pub adjust_reason: StockReason,
    pub adjust_note: String,

    // reports
    pub period: Period,
    pub z: Option<ZReport>,
    pub top: Vec<TopProduct>,
    pub daily: Vec<DaySummary>,

    // settings
    pub store_info: StoreInfo,
    pub cfg_transport: usize,
    pub cfg_host: String,
    pub cfg_port: String,
    pub cfg_device: String,
    pub cfg_paper_mm: u32,
    pub cfg_auto_cut: bool,
    pub cfg_drawer: bool,
    pub cfg_scan_gap: String,

    // users
    pub users: Vec<User>,
    pub user_form: Option<UserForm>,
    pub pin_reset: Option<(i64, String)>,
}

impl PosApp {
    pub fn new(db: Db, data_dir: PathBuf) -> Self {
        let lang = db
            .get_setting(pos_store::settings::KEY_LANGUAGE)
            .ok()
            .flatten()
            .map(|c| Lang::from_code(&c))
            .unwrap_or_default();

        let store_info = db.store_info().unwrap_or_default();
        let cfg_transport = match db
            .get_setting_or(pos_store::settings::KEY_PRINTER_TRANSPORT, "preview")
            .unwrap_or_default()
            .as_str()
        {
            "file" => 1,
            "network" => 2,
            _ => 0,
        };
        let cfg_host = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_HOST, "192.168.0.50")
            .unwrap_or_default();
        let cfg_port = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_PORT, "9100")
            .unwrap_or_default();
        let cfg_device = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_DEVICE, "/dev/usb/lp0")
            .unwrap_or_default();
        let cfg_paper_mm = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_PAPER_WIDTH, "80")
            .unwrap_or_default()
            .parse()
            .unwrap_or(80);
        let cfg_auto_cut = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_AUTO_CUT, "1")
            .unwrap_or_default()
            != "0";
        let cfg_drawer = db
            .get_setting_or(pos_store::settings::KEY_PRINTER_DRAWER_KICK, "1")
            .unwrap_or_default()
            != "0";
        let cfg_scan_gap = db
            .get_setting_or(pos_store::settings::KEY_SCAN_MAX_GAP_MS, "30")
            .unwrap_or_default();
        let scan_gap_ms = cfg_scan_gap.parse().unwrap_or(30);

        let has_users = db.has_users().unwrap_or(false);

        let mut app = Self {
            db,
            data_dir,
            lang,
            session: None,
            has_users,
            screen: Screen::Register,
            toast: None,
            login_username: String::new(),
            login_pin: String::new(),
            login_error: None,
            login_attempts: 0,
            fr_username: String::new(),
            fr_pin: String::new(),
            fr_pin2: String::new(),
            fr_error: None,
            cart: Cart::new(),
            search: String::new(),
            results: Vec::new(),
            scan: ScanBuffer::default(),
            scan_gap_ms,
            simulate: String::new(),
            barcode_msg: None,
            pay: None,
            receipt_view: None,
            create_for_barcode: None,
            products: Vec::new(),
            product_filter: String::new(),
            show_inactive: false,
            product_form: None,
            stock: Vec::new(),
            low_only: false,
            threshold: 5,
            inv_selected: None,
            movements: Vec::new(),
            adjust_delta: String::new(),
            adjust_reason: StockReason::Purchase,
            adjust_note: String::new(),
            period: Period::Today,
            z: None,
            top: Vec::new(),
            daily: Vec::new(),
            store_info,
            cfg_transport,
            cfg_host,
            cfg_port,
            cfg_device,
            cfg_paper_mm,
            cfg_auto_cut,
            cfg_drawer,
            cfg_scan_gap,
            users: Vec::new(),
            user_form: None,
            pin_reset: None,
        };
        app.reload_products();
        app.reload_stock();
        app.reload_users();
        app
    }

    pub(crate) fn t(&self, key: Key) -> &'static str {
        t(self.lang, key)
    }

    pub(crate) fn rtl(&self) -> bool {
        self.lang.is_rtl()
    }

    pub(crate) fn is_admin(&self) -> bool {
        self.session
            .as_ref()
            .map(|u| u.role.is_admin())
            .unwrap_or(false)
    }

    pub(crate) fn toast_ok(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast::new(msg.into(), false));
    }

    pub(crate) fn toast_err(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast::new(msg.into(), true));
    }

    pub(crate) fn store_error(&mut self, e: StoreError) {
        self.toast_err(e.to_string());
    }

    pub(crate) fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        let _ = self
            .db
            .set_setting(pos_store::settings::KEY_LANGUAGE, lang.code());
    }

    pub(crate) fn printer_config(&self) -> PrinterConfig {
        let transport = match self.cfg_transport {
            1 => Transport::File(self.cfg_device.clone()),
            2 => Transport::Network(format!("{}:{}", self.cfg_host.trim(), self.cfg_port.trim())),
            _ => Transport::Preview,
        };
        PrinterConfig {
            transport,
            paper_width: PaperWidth::from_mm(self.cfg_paper_mm),
            auto_cut: self.cfg_auto_cut,
            drawer_kick: self.cfg_drawer,
        }
    }

    pub(crate) fn reload_products(&mut self) {
        self.products = self
            .db
            .list_products(self.show_inactive)
            .unwrap_or_default();
    }

    pub(crate) fn reload_stock(&mut self) {
        let rows = if self.low_only {
            self.db.low_stock(self.threshold)
        } else {
            self.db.stock_overview()
        };
        self.stock = rows.unwrap_or_default();
    }

    pub(crate) fn reload_users(&mut self) {
        self.users = self.db.list_users().unwrap_or_default();
    }

    // ---- navigation -------------------------------------------------------

    fn nav_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let screens = [
                Screen::Register,
                Screen::Products,
                Screen::Inventory,
                Screen::Reports,
                Screen::Settings,
                Screen::Users,
            ];
            let mut ordered: Vec<Screen> = screens
                .into_iter()
                .filter(|s| !s.admin_only() || self.is_admin())
                .collect();
            if self.rtl() {
                ordered.reverse();
            }
            for s in ordered {
                if ui
                    .selectable_label(self.screen == s, self.t(s.key()))
                    .clicked()
                {
                    self.screen = s;
                    match s {
                        Screen::Products => self.reload_products(),
                        Screen::Inventory => self.reload_stock(),
                        Screen::Users => self.reload_users(),
                        Screen::Reports => self.refresh_reports(),
                        _ => {}
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let name = self
                    .session
                    .as_ref()
                    .map(|u| format!("{} ({})", u.username, u.role))
                    .unwrap_or_default();
                if ui.button(self.t(Key::Logout)).clicked() {
                    self.session = None;
                    self.cart.clear();
                    self.screen = Screen::Register;
                }
                ui.label(name);
                // language switch
                for l in Lang::ALL {
                    if ui
                        .selectable_label(self.lang == l, l.native_name())
                        .clicked()
                    {
                        self.set_lang(l);
                    }
                }
            });
        });
    }

    fn main_screen(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("nav").show(ui, |ui| {
            ui.add_space(4.0);
            self.nav_bar(ui);
            ui.add_space(4.0);
        });

        if let Some(toast) = &self.toast {
            if toast.expired() {
                self.toast = None;
            } else {
                let color = if toast.is_error {
                    egui::Color32::from_rgb(180, 40, 40)
                } else {
                    egui::Color32::from_rgb(30, 120, 60)
                };
                egui::Panel::bottom("toast").show(ui, |ui| {
                    ui.colored_label(color, &toast.message);
                });
            }
        }

        egui::CentralPanel::default().show(ui, |ui| match self.screen {
            Screen::Register => self.register_screen(ui),
            Screen::Products => self.products_screen(ui),
            Screen::Inventory => self.inventory_screen(ui),
            Screen::Reports => self.reports_screen(ui),
            Screen::Settings => self.settings_screen(ui),
            Screen::Users => self.users_screen(ui),
        });

        // Modal dialogs (floating).
        self.register_dialogs(ui.ctx());
        self.products_dialogs(ui.ctx());
        self.settings_dialogs(ui.ctx());
        self.users_dialogs(ui.ctx());
    }

    /// Feed the global barcode scan buffer from raw input events.
    fn handle_scan_input(&mut self, ctx: &egui::Context) {
        if self.session.is_none() {
            return;
        }
        // Only capture when no text field has focus (avoids double handling).
        if ctx.memory(|m| m.focused()).is_some() {
            self.scan.reset();
            return;
        }
        let threshold = self.scan_gap_ms;
        let completed = ctx.input(|i| {
            let now = i.time;
            let mut done: Option<String> = None;
            for event in &i.events {
                match event {
                    egui::Event::Text(text) => {
                        for ch in text.chars() {
                            if ch.is_ascii_graphic() || ch == ' ' {
                                self.scan.push_char(ch, now, threshold);
                            }
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        if let Some(code) = self.scan.take() {
                            done = Some(code);
                        }
                    }
                    _ => {}
                }
            }
            done
        });
        if let Some(code) = completed {
            self.handle_barcode(code);
        }
    }
}

impl eframe::App for PosApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_scan_input(ui.ctx());
        if !self.has_users {
            self.first_run_screen(ui);
            return;
        }
        if self.session.is_none() {
            self.login_screen(ui);
            return;
        }
        self.main_screen(ui);
    }
}
