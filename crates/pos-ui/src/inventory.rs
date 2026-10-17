//! Inventory screen: stock levels and manual adjustments with a movement ledger.

use eframe::egui;
use pos_domain::StockReason;
use pos_i18n::{t, Key};

use crate::util::{fmt_ts, hrow};
use crate::PosApp;

impl PosApp {
    pub fn inventory_screen(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading(self.t(Key::NavInventory));
        ui.add_space(4.0);

        hrow(ui, self.rtl(), |ui| {
            ui.label(self.t(Key::Threshold));
            let mut changed = false;
            if ui
                .add(egui::DragValue::new(&mut self.threshold).range(0..=10000))
                .changed()
            {
                changed = true;
            }
            let low_label = self.t(Key::LowStockOnly).to_string();
            if ui.checkbox(&mut self.low_only, low_label).changed() {
                changed = true;
            }
            if ui.button(self.t(Key::Search)).clicked() {
                changed = true;
            }
            if changed {
                self.reload_stock();
            }
        });
        ui.separator();

        if self.stock.is_empty() {
            ui.add_space(20.0);
            ui.weak(self.t(Key::NoData));
            return;
        }

        let mut select: Option<i64> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("stock_grid")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.t(Key::Name));
                    ui.strong(self.t(Key::OnHand));
                    ui.strong("");
                    ui.end_row();
                    for row in &self.stock {
                        ui.label(row.product.display_name(self.rtl()));
                        let color = if row.quantity <= self.threshold {
                            egui::Color32::from_rgb(190, 110, 20)
                        } else {
                            egui::Color32::PLACEHOLDER
                        };
                        ui.colored_label(color, row.quantity.to_string());
                        if ui.small_button(self.t(Key::Adjust)).clicked() {
                            select = Some(row.product.id);
                        }
                        ui.end_row();
                    }
                });
        });

        if let Some(id) = select {
            self.inv_selected = Some(id);
            self.movements = self.db.movements_for(id).unwrap_or_default();
            self.adjust_delta = "0".into();
            self.adjust_note.clear();
        }

        self.inventory_dialog(ui.ctx());
    }

    fn inventory_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.inv_selected else { return };
        let lang = self.lang;
        let rtl = self.rtl();
        let product = self.db.get_product(id).ok().flatten();
        let on_hand = self.db.stock_for(id).unwrap_or(0);

        let mut open = true;
        let mut apply = false;

        egui::Window::new(format!(
            "{}: {}",
            t(lang, Key::Adjust),
            product.as_ref().map(|p| p.name.as_str()).unwrap_or("")
        ))
        .collapsible(false)
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(format!("{}: {on_hand}", t(lang, Key::OnHand)));
            ui.separator();
            hrow(ui, rtl, |ui| {
                ui.label(t(lang, Key::QuantityDelta));
                ui.add(egui::TextEdit::singleline(&mut self.adjust_delta).desired_width(80.0));
            });
            hrow(ui, rtl, |ui| {
                ui.label(t(lang, Key::Reason));
                let reasons = [
                    (StockReason::Purchase, t(lang, Key::ReasonPurchase)),
                    (StockReason::Adjustment, t(lang, Key::ReasonAdjustment)),
                    (StockReason::Return, t(lang, Key::ReasonReturn)),
                ];
                for (r, label) in reasons {
                    if ui
                        .selectable_label(self.adjust_reason == r, label)
                        .clicked()
                    {
                        self.adjust_reason = r;
                    }
                }
            });
            hrow(ui, rtl, |ui| {
                ui.label(t(lang, Key::Note));
                ui.text_edit_singleline(&mut self.adjust_note);
            });
            ui.add_space(6.0);
            if ui.button(t(lang, Key::Apply)).clicked() {
                apply = true;
            }

            ui.separator();
            ui.strong(t(lang, Key::MovementHistory));
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    if self.movements.is_empty() {
                        ui.weak(t(lang, Key::NoData));
                    }
                    for m in &self.movements {
                        let reason = match m.reason {
                            StockReason::Sale => t(lang, Key::ReasonSale),
                            StockReason::Purchase => t(lang, Key::ReasonPurchase),
                            StockReason::Adjustment => t(lang, Key::ReasonAdjustment),
                            StockReason::Return => t(lang, Key::ReasonReturn),
                        };
                        ui.label(format!(
                            "{}  {:>+4}  {}  {}",
                            fmt_ts(m.created_at),
                            m.delta,
                            reason,
                            m.note.clone().unwrap_or_default()
                        ));
                    }
                });
        });

        if apply {
            let delta = self.adjust_delta.trim().parse::<i64>().unwrap_or_else(|_| {
                // Accept signed numbers with a leading '+'.
                self.adjust_delta
                    .trim()
                    .trim_start_matches('+')
                    .parse::<i64>()
                    .unwrap_or(0)
            });
            let user = self.session.as_ref().map(|u| u.id);
            let note = if self.adjust_note.trim().is_empty() {
                None
            } else {
                Some(self.adjust_note.clone())
            };
            match self
                .db
                .adjust_stock(id, delta, self.adjust_reason, note.as_deref(), user)
            {
                Ok(new_level) => {
                    self.movements = self.db.movements_for(id).unwrap_or_default();
                    self.reload_stock();
                    self.toast_ok(format!("{}: {new_level}", t(lang, Key::OnHand)));
                }
                Err(e) => self.store_error(e),
            }
        }
        if !open {
            self.inv_selected = None;
        }
    }
}
