//! The Register / checkout screen: scan or search, cart, discounts, pay.
//!
//! Scan-buffer → barcode handling is implemented here; the raw event capture
//! lives in `PosApp::handle_scan_input`.

use eframe::egui;
use pos_domain::{CartLine, Discount, Money, PaymentMethod, Product};
use pos_i18n::{t, Key};
use pos_printing::ReceiptDoc;

use crate::util::{hrow, money};
use crate::{PayState, PosApp};

impl PosApp {
    pub fn register_screen(&mut self, ui: &mut egui::Ui) {
        // Cart on the right in LTR (classic POS layout), mirrored for RTL.
        let (side, center, side_width) = if self.rtl() {
            (
                egui::Panel::right("scan_panel"),
                egui::CentralPanel::default(),
                360.0,
            )
        } else {
            (
                egui::Panel::left("scan_panel"),
                egui::CentralPanel::default(),
                360.0,
            )
        };
        side.resizable(false)
            .default_size(side_width)
            .show(ui, |ui| self.scan_panel(ui));
        center.show(ui, |ui| self.cart_panel(ui));
    }

    fn scan_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading(self.t(Key::ScanOrSearch));

        let scan_hint = self.t(Key::ScanOrSearch).to_string();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(scan_hint)
                .desired_width(f32::INFINITY),
        );
        if resp.changed() {
            self.update_results();
        }

        // Simulate scan (testability without hardware).
        ui.add_space(6.0);
        hrow(ui, self.rtl(), |ui| {
            let sim_hint = self.t(Key::SimulateScan);
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.simulate)
                    .hint_text(sim_hint)
                    .desired_width(180.0),
            );
            let submit = ui.button(self.t(Key::Enter)).clicked()
                || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if submit && !self.simulate.trim().is_empty() {
                let code = std::mem::take(&mut self.simulate);
                self.handle_barcode(code.trim().to_string());
            }
        });

        if let Some((msg, is_err)) = self.barcode_msg.clone() {
            let color = if is_err {
                egui::Color32::from_rgb(180, 40, 40)
            } else {
                egui::Color32::from_rgb(30, 120, 60)
            };
            ui.colored_label(color, msg);
        }

        if let Some(code) = self.create_for_barcode.clone() {
            ui.add_space(4.0);
            ui.label(format!("{}: {code}", self.t(Key::NotInCatalogue)));
            if ui.button(self.t(Key::CreateProductFor)).clicked() {
                self.product_form = Some(crate::ProductForm {
                    id: None,
                    sku: String::new(),
                    barcode: code.clone(),
                    name: String::new(),
                    name_ar: String::new(),
                    price: "0,00".into(),
                    tax_index: 0,
                    unit: "pcs".into(),
                    category_id: None,
                    is_active: true,
                });
                self.screen = crate::Screen::Products;
                self.create_for_barcode = None;
            }
        }

        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.results.is_empty() {
                ui.weak(self.t(Key::NoResults));
            }
            let mut to_add: Option<Product> = None;
            for p in &self.results {
                let label = format!("{}   {}", p.display_name(self.rtl()), money(p.price));
                if ui
                    .add(egui::Button::new(label).min_size(egui::vec2(ui.available_width(), 32.0)))
                    .clicked()
                {
                    to_add = Some(p.clone());
                }
            }
            if let Some(p) = to_add {
                self.add_product_to_cart(&p);
            }
        });
    }

    fn update_results(&mut self) {
        let q = self.search.trim();
        if q.is_empty() {
            self.results.clear();
            return;
        }
        self.results = self.db.search_products(q, 30).unwrap_or_default();
    }

    fn add_product_to_cart(&mut self, p: &Product) {
        let line = CartLine::new(
            Some(p.id),
            p.display_name(self.rtl()).to_string(),
            p.price,
            p.tax_rate,
        );
        self.cart.add_or_increment(line);
        self.barcode_msg = None;
        self.create_for_barcode = None;
    }

    /// Called when a barcode is scanned or submitted.
    pub(crate) fn handle_barcode(&mut self, code: String) {
        match self.db.find_by_barcode(&code) {
            Ok(Some(p)) => {
                self.add_product_to_cart(&p);
                self.toast_ok(format!(
                    "{}: {}",
                    self.t(Key::Item),
                    p.display_name(self.rtl())
                ));
            }
            Ok(None) => {
                self.barcode_msg = Some((format!("{}: {code}", self.t(Key::NotInCatalogue)), true));
                self.create_for_barcode = Some(code);
            }
            Err(e) => self.store_error(e),
        }
    }

    fn cart_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        hrow(ui, self.rtl(), |ui| {
            ui.heading(self.t(Key::Cart));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t(Key::NewSale)).clicked() {
                    self.cart.clear();
                }
            });
        });

        if self.cart.is_empty() {
            ui.add_space(20.0);
            ui.weak(self.t(Key::EmptyCart));
            return;
        }

        // Column headers.
        ui.separator();
        let mut remove_idx: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 260.0)
            .show(ui, |ui| {
                let mut qty_changes: Vec<(usize, i64)> = Vec::new();
                let mut disc_changes: Vec<(usize, u32)> = Vec::new();
                let mut shown = Vec::new();

                for (i, line) in self.cart.lines.iter().enumerate() {
                    let unit = line.unit_price;
                    let total = line.total();
                    shown.push((i, line.name.clone(), line.qty, unit, total, line.discount));
                }

                for (i, name, qty, unit, total, disc) in shown {
                    hrow(ui, self.rtl(), |ui| {
                        ui.add_sized(
                            egui::vec2(ui.available_width() - 340.0, 24.0),
                            egui::Label::new(&name).truncate(),
                        );
                        if ui.small_button("−").clicked() {
                            qty_changes.push((i, -1));
                        }
                        ui.label(format!("{qty}"));
                        if ui.small_button("+").clicked() {
                            qty_changes.push((i, 1));
                        }
                        ui.label(money(unit));
                        let mut pct = match disc {
                            Discount::PercentBp(bp) => bp / 100,
                            _ => 0,
                        };
                        if ui
                            .add(egui::DragValue::new(&mut pct).range(0..=90).speed(1.0))
                            .changed()
                        {
                            disc_changes.push((i, pct));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").clicked() {
                                remove_idx = Some(i);
                            }
                            ui.label(egui::RichText::new(money(total)).strong());
                        });
                    });
                }

                for (i, d) in qty_changes {
                    let q = self.cart.lines[i].qty + d;
                    self.cart.set_qty(i, q.max(1));
                }
                for (i, pct) in disc_changes {
                    if let Some(line) = self.cart.lines.get_mut(i) {
                        line.discount = if pct == 0 {
                            Discount::None
                        } else {
                            Discount::PercentBp(pct * 100)
                        };
                    }
                }
            });

        if let Some(i) = remove_idx {
            self.cart.remove(i);
        }

        ui.separator();
        self.totals_panel(ui);
    }

    fn totals_panel(&mut self, ui: &mut egui::Ui) {
        let totals = self.cart.totals();

        // Sale-level discount controls.
        hrow(ui, self.rtl(), |ui| {
            ui.label(self.t(Key::SaleDiscount));
            let is_pct = matches!(
                self.cart.sale_discount,
                Discount::PercentBp(_) | Discount::None
            );
            if ui.selectable_label(is_pct, self.t(Key::Percent)).clicked() {
                self.cart.sale_discount = Discount::None;
            }
            if ui.selectable_label(!is_pct, self.t(Key::Amount)).clicked() {
                self.cart.sale_discount = Discount::Amount(Money::ZERO);
            }
            match self.cart.sale_discount {
                Discount::PercentBp(bp) => {
                    let mut pct = bp / 100;
                    if ui
                        .add(egui::DragValue::new(&mut pct).range(0..=90).speed(1.0))
                        .changed()
                    {
                        self.cart.sale_discount = if pct == 0 {
                            Discount::None
                        } else {
                            Discount::PercentBp(pct * 100)
                        };
                    }
                }
                Discount::Amount(m) => {
                    let mut s = m.format_amount();
                    if ui
                        .add(egui::TextEdit::singleline(&mut s).desired_width(80.0))
                        .changed()
                    {
                        self.cart.sale_discount =
                            Discount::Amount(Money::parse(&s).unwrap_or(Money::ZERO));
                    }
                }
                Discount::None => {}
            }
        });

        ui.separator();
        let mut rows: Vec<(String, String, bool)> = Vec::new();
        rows.push((
            self.t(Key::SubtotalHt).to_string(),
            money(totals.subtotal),
            false,
        ));
        if !totals.discount_total.is_zero() {
            rows.push((
                self.t(Key::Discounts).to_string(),
                format!("-{}", money(totals.discount_total)),
                false,
            ));
        }
        for tl in &totals.tax_lines {
            if tl.tax.is_zero() {
                continue;
            }
            rows.push((
                format!("{} {}", self.t(Key::Vat), tl.rate.label()),
                money(tl.tax),
                false,
            ));
        }
        rows.push((self.t(Key::TotalTtc).to_string(), money(totals.total), true));

        for (label, value, bold) in rows {
            hrow(ui, self.rtl(), |ui| {
                let l = if bold {
                    egui::RichText::new(label).strong().size(18.0)
                } else {
                    egui::RichText::new(label)
                };
                let v = if bold {
                    egui::RichText::new(value).strong().size(18.0)
                } else {
                    egui::RichText::new(value)
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(v);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(l);
                    });
                });
            });
        }

        ui.add_space(8.0);
        let can_pay = !self.cart.is_empty();
        if ui
            .add_enabled(
                can_pay,
                egui::Button::new(egui::RichText::new(self.t(Key::Pay)).size(20.0))
                    .min_size(egui::vec2(ui.available_width(), 44.0)),
            )
            .clicked()
        {
            self.pay = Some(PayState {
                method: 0,
                tendered: totals.total.format_amount(),
            });
        }
    }

    pub fn register_dialogs(&mut self, ctx: &egui::Context) {
        self.pay_dialog(ctx);
        self.receipt_dialog(ctx);
    }

    fn pay_dialog(&mut self, ctx: &egui::Context) {
        if self.pay.is_none() {
            return;
        }
        let lang = self.lang;
        let rtl = self.rtl();
        let total = self.cart.totals().total;

        let mut open = true;
        let mut confirm = false;
        let mut cancel = false;
        let mut chosen_method = 0usize;
        let mut chosen_tendered = String::new();

        {
            let pay = self.pay.as_mut().unwrap();
            egui::Window::new(t(lang, Key::Payment))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    hrow(ui, rtl, |ui| {
                        for (idx, key) in [(0usize, Key::Cash), (1, Key::Card), (2, Key::Other)] {
                            if ui
                                .selectable_label(pay.method == idx, t(lang, key))
                                .clicked()
                            {
                                pay.method = idx;
                            }
                        }
                    });

                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {}",
                            t(lang, Key::TotalTtc),
                            money(total)
                        ))
                        .strong(),
                    );

                    let tendered = Money::parse(&pay.tendered).unwrap_or(Money::ZERO);
                    let effective = if pay.method == 0 { tendered } else { total };
                    let change =
                        (effective - total).clamp_to(Money::ZERO, Money::from_centimes(i64::MAX));

                    if pay.method == 0 {
                        hrow(ui, rtl, |ui| {
                            ui.label(t(lang, Key::Tendered));
                            ui.add(
                                egui::TextEdit::singleline(&mut pay.tendered).desired_width(120.0),
                            );
                        });
                        if tendered < total {
                            ui.colored_label(
                                egui::Color32::from_rgb(180, 40, 40),
                                t(lang, Key::InsufficientAmount),
                            );
                        }
                    }

                    hrow(ui, rtl, |ui| {
                        ui.label(t(lang, Key::Change));
                        ui.label(egui::RichText::new(money(change)).strong());
                    });

                    ui.add_space(8.0);
                    hrow(ui, rtl, |ui| {
                        let ok = pay.method != 0 || tendered >= total;
                        if ui
                            .add_enabled(ok, egui::Button::new(t(lang, Key::Confirm)))
                            .clicked()
                        {
                            confirm = true;
                            chosen_method = pay.method;
                            chosen_tendered = pay.tendered.clone();
                        }
                        if ui.button(t(lang, Key::Cancel)).clicked() {
                            cancel = true;
                        }
                    });
                });
        }

        if confirm {
            let method = match chosen_method {
                1 => PaymentMethod::Card,
                2 => PaymentMethod::Other,
                _ => PaymentMethod::Cash,
            };
            let tendered = if method == PaymentMethod::Cash {
                Money::parse(&chosen_tendered).unwrap_or(total)
            } else {
                total
            };
            self.pay = None;
            self.finalize_sale(method, tendered);
        } else if cancel || !open {
            self.pay = None;
        }
    }

    fn finalize_sale(&mut self, method: PaymentMethod, tendered: Money) {
        let cashier_id = match &self.session {
            Some(u) => u.id,
            None => return,
        };
        let cart = self.cart.clone();
        match self.db.create_sale(
            &cart,
            cashier_id,
            pos_store::sales::PaymentInfo { method, tendered },
        ) {
            Ok(sale) => {
                let doc = ReceiptDoc::from_sale(
                    &sale,
                    &self.store_info,
                    self.printer_config().paper_width,
                );
                let text = pos_printing::render_text(&doc);
                self.receipt_view = Some((doc, text));
                self.cart.clear();
                self.search.clear();
                self.results.clear();
                self.reload_stock();
                self.toast_ok(format!("{} {}", self.t(Key::Saved), sale.sale.receipt_no));
            }
            Err(e) => self.store_error(e),
        }
    }

    fn receipt_dialog(&mut self, ctx: &egui::Context) {
        let Some((doc, text)) = self.receipt_view.clone() else {
            return;
        };
        let lang = self.lang;
        let rtl = self.rtl();
        let mut open = true;
        let mut close = false;
        let mut print = false;

        egui::Window::new(t(lang, Key::Receipt))
            .collapsible(false)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&text).monospace()).wrap());
                    });
                ui.add_space(6.0);
                hrow(ui, rtl, |ui| {
                    if ui.button(t(lang, Key::Print)).clicked() {
                        print = true;
                    }
                    if ui.button(t(lang, Key::Close)).clicked() {
                        close = true;
                    }
                });
            });

        if print {
            match self.printer_config().print(&doc) {
                Ok(()) => self.toast_ok(self.t(Key::Saved)),
                Err(e) => self.toast_err(format!("{}: {e}", self.t(Key::PrintFailed))),
            }
        }
        if close || !open {
            self.receipt_view = None;
        }
    }
}
