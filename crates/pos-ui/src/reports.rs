//! Reports screen: period summaries, TVA by rate, top products and the Z-report.

use eframe::egui;
use pos_i18n::Key;

use crate::util::{hrow, local_day_start, local_month_start, money, now_plus};
use crate::{Period, PosApp};

impl PosApp {
    fn period_bounds(&self) -> (i64, i64) {
        match self.period {
            Period::Today => (local_day_start(0), now_plus()),
            Period::Week => (local_day_start(6), now_plus()),
            Period::Month => (local_month_start(), now_plus()),
        }
    }

    pub fn refresh_reports(&mut self) {
        let (from, to) = self.period_bounds();
        self.z = self.db.z_report(from, to).ok();
        self.top = self.db.top_products(from, to, 10).unwrap_or_default();
        self.daily = self.db.daily_summary(from, to).unwrap_or_default();
    }

    pub fn reports_screen(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        hrow(ui, self.rtl(), |ui| {
            ui.heading(self.t(Key::NavReports));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t(Key::Search)).clicked() {
                    self.refresh_reports();
                }
                for (p, key) in [
                    (Period::Today, Key::Today),
                    (Period::Week, Key::ThisWeek),
                    (Period::Month, Key::ThisMonth),
                ] {
                    if ui.selectable_label(self.period == p, self.t(key)).clicked() {
                        self.period = p;
                        self.refresh_reports();
                    }
                }
            });
        });
        ui.separator();

        if self.z.is_none() {
            self.refresh_reports();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(z) = self.z.clone() {
                egui::Grid::new("z_summary")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(self.t(Key::ZReport));
                        ui.strong("");
                        ui.end_row();
                        ui.label(self.t(Key::SalesCount));
                        ui.label(z.count.to_string());
                        ui.end_row();
                        ui.label(self.t(Key::Gross));
                        ui.label(money(z.gross));
                        ui.end_row();
                        ui.label(self.t(Key::Discounts));
                        ui.label(format!("-{}", money(z.discount)));
                        ui.end_row();
                        ui.label(self.t(Key::VatCollected));
                        ui.label(money(z.tax_total));
                        ui.end_row();
                        ui.label(self.t(Key::NetTotal));
                        ui.label(egui::RichText::new(money(z.total)).strong());
                        ui.end_row();
                        ui.label(self.t(Key::Cash));
                        ui.label(money(z.cash_total));
                        ui.end_row();
                        ui.label(self.t(Key::Card));
                        ui.label(money(z.card_total));
                        ui.end_row();
                        ui.label(self.t(Key::Refunds));
                        ui.label(money(z.refunds_total));
                        ui.end_row();
                        ui.label(self.t(Key::CashExpected));
                        ui.label(egui::RichText::new(money(z.cash_expected())).strong());
                        ui.end_row();
                    });

                ui.add_space(12.0);
                ui.strong(self.t(Key::ByRate));
                egui::Grid::new("by_rate")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(self.t(Key::TaxRate));
                        ui.strong(self.t(Key::Base));
                        ui.strong(self.t(Key::Vat));
                        ui.end_row();
                        for r in &z.by_rate {
                            ui.label(r.rate.label());
                            ui.label(money(r.base));
                            ui.label(money(r.tax));
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(12.0);
            ui.strong(self.t(Key::TopProducts));
            egui::Grid::new("top_products")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.t(Key::Item));
                    ui.strong(self.t(Key::Qty));
                    ui.strong(self.t(Key::LineTotal));
                    ui.end_row();
                    if self.top.is_empty() {
                        ui.weak(self.t(Key::NoData));
                        ui.end_row();
                    }
                    for p in &self.top {
                        ui.label(&p.name);
                        ui.label(p.qty.to_string());
                        ui.label(money(p.total));
                        ui.end_row();
                    }
                });

            ui.add_space(12.0);
            ui.strong(self.t(Key::DailyTotals));
            egui::Grid::new("daily")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.t(Key::Day));
                    ui.strong(self.t(Key::SalesCount));
                    ui.strong(self.t(Key::VatCollected));
                    ui.strong(self.t(Key::NetTotal));
                    ui.strong(self.t(Key::Cash));
                    ui.end_row();
                    if self.daily.is_empty() {
                        ui.weak(self.t(Key::NoData));
                        ui.end_row();
                    }
                    for d in &self.daily {
                        ui.label(&d.day);
                        ui.label(d.count.to_string());
                        ui.label(money(d.tax_total));
                        ui.label(money(d.total));
                        ui.label(money(d.cash));
                        ui.end_row();
                    }
                });
        });
    }
}
