//! Products screen and the add/edit dialog.

use eframe::egui;
use pos_domain::{Money, Product, TaxRate};
use pos_i18n::{t, Key};
use pos_store::products::NewProduct;

use crate::util::hrow;
use crate::{PosApp, ProductForm};

impl PosApp {
    pub fn products_screen(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        hrow(ui, self.rtl(), |ui| {
            ui.heading(self.t(Key::Products));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t(Key::NewProduct)).clicked() {
                    self.product_form = Some(ProductForm {
                        id: None,
                        sku: String::new(),
                        barcode: String::new(),
                        name: String::new(),
                        name_ar: String::new(),
                        price: "0,00".into(),
                        tax_index: 0,
                        unit: "pcs".into(),
                        category_id: None,
                        is_active: true,
                    });
                }
            });
        });

        ui.add_space(4.0);
        hrow(ui, self.rtl(), |ui| {
            let search_hint = self.t(Key::Search);
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.product_filter)
                    .hint_text(search_hint)
                    .desired_width(240.0),
            );
            if r.changed() {
                // filter is applied locally below
            }
            let inactive_label = self.t(Key::ShowInactive);
            if ui
                .checkbox(&mut self.show_inactive, inactive_label)
                .changed()
            {
                self.reload_products();
            }
        });
        ui.separator();

        let filter = self.product_filter.trim().to_lowercase();
        let rows: Vec<Product> = self
            .products
            .iter()
            .filter(|p| {
                filter.is_empty()
                    || p.name.to_lowercase().contains(&filter)
                    || p.name_ar.contains(&filter)
                    || p.sku.to_lowercase().contains(&filter)
                    || p.barcode.as_deref().unwrap_or("").contains(&filter)
            })
            .cloned()
            .collect();

        if self.products.is_empty() {
            ui.add_space(20.0);
            ui.weak(self.t(Key::NoData));
        }

        let mut edit: Option<usize> = None;
        let mut toggle: Option<i64> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("products_grid")
                .num_columns(6)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(self.t(Key::Name));
                    ui.strong(self.t(Key::Barcode));
                    ui.strong(self.t(Key::Price));
                    ui.strong(self.t(Key::TaxRate));
                    ui.strong(self.t(Key::Stock));
                    ui.strong("");
                    ui.end_row();

                    for p in &rows {
                        ui.label(p.display_name(self.rtl()));
                        ui.label(p.barcode.clone().unwrap_or_default());
                        ui.label(p.price.format_dzd());
                        ui.label(p.tax_rate.label());
                        ui.label(if p.is_active { "" } else { "—" });
                        hrow(ui, self.rtl(), |ui| {
                            if ui.small_button(self.t(Key::Edit)).clicked() {
                                // find index in self.products
                                if let Some(idx) = self.products.iter().position(|x| x.id == p.id) {
                                    edit = Some(idx);
                                }
                            }
                            let toggle_label = if p.is_active {
                                self.t(Key::Deactivate)
                            } else {
                                self.t(Key::Activate)
                            };
                            if ui.small_button(toggle_label).clicked() {
                                toggle = Some(p.id);
                            }
                        });
                        ui.end_row();
                    }
                });
        });

        if let Some(idx) = edit {
            let p = self.products[idx].clone();
            self.product_form = Some(ProductForm {
                id: Some(p.id),
                sku: p.sku.clone(),
                barcode: p.barcode.clone().unwrap_or_default(),
                name: p.name.clone(),
                name_ar: p.name_ar.clone(),
                price: p.price.format_amount(),
                tax_index: TaxRate::ALL
                    .iter()
                    .position(|r| *r == p.tax_rate)
                    .unwrap_or(0),
                unit: p.unit.clone(),
                category_id: p.category_id,
                is_active: p.is_active,
            });
        }
        if let Some(id) = toggle {
            let active = self
                .products
                .iter()
                .find(|p| p.id == id)
                .map(|p| !p.is_active)
                .unwrap_or(true);
            if let Err(e) = self.db.set_product_active(id, active) {
                self.store_error(e);
            }
            self.reload_products();
        }
    }

    pub fn products_dialogs(&mut self, ctx: &egui::Context) {
        if self.product_form.is_none() {
            return;
        }
        let lang = self.lang;
        let rtl = self.rtl();
        let categories = self.db.list_categories().unwrap_or_default();
        let title = if self.product_form.as_ref().and_then(|f| f.id).is_some() {
            t(lang, Key::EditProduct)
        } else {
            t(lang, Key::NewProduct)
        };

        let mut open = true;
        let mut save = false;
        let mut cancel = false;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                let form = self.product_form.as_mut().unwrap();
                egui::Grid::new("product_form")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label(t(lang, Key::Name));
                        ui.text_edit_singleline(&mut form.name);
                        ui.end_row();

                        ui.label(t(lang, Key::NameAr));
                        ui.text_edit_singleline(&mut form.name_ar);
                        ui.end_row();

                        ui.label(t(lang, Key::Sku));
                        ui.text_edit_singleline(&mut form.sku);
                        ui.end_row();

                        ui.label(t(lang, Key::Barcode));
                        ui.text_edit_singleline(&mut form.barcode);
                        ui.end_row();

                        ui.label(t(lang, Key::Price));
                        ui.add(egui::TextEdit::singleline(&mut form.price).desired_width(120.0));
                        ui.end_row();

                        ui.label(t(lang, Key::TaxRate));
                        egui::ComboBox::from_id_salt("tax")
                            .selected_text(TaxRate::ALL[form.tax_index].label())
                            .show_ui(ui, |ui| {
                                for (i, rate) in TaxRate::ALL.iter().enumerate() {
                                    ui.selectable_value(&mut form.tax_index, i, rate.label());
                                }
                            });
                        ui.end_row();

                        ui.label(t(lang, Key::Category));
                        let cat_name = form
                            .category_id
                            .and_then(|id| categories.iter().find(|c| c.id == id))
                            .map(|c| c.name.clone())
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt("cat")
                            .selected_text(if cat_name.is_empty() {
                                "-".to_string()
                            } else {
                                cat_name
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut form.category_id, None, "-");
                                for c in &categories {
                                    ui.selectable_value(&mut form.category_id, Some(c.id), &c.name);
                                }
                            });
                        ui.end_row();

                        ui.label(t(lang, Key::Unit));
                        ui.text_edit_singleline(&mut form.unit);
                        ui.end_row();

                        ui.label(t(lang, Key::Active));
                        ui.checkbox(&mut form.is_active, "");
                        ui.end_row();
                    });

                ui.add_space(8.0);
                hrow(ui, rtl, |ui| {
                    if ui.button(t(lang, Key::Save)).clicked() {
                        save = true;
                    }
                    if ui.button(t(lang, Key::Cancel)).clicked() {
                        cancel = true;
                    }
                });
            });

        if save {
            self.save_product_form();
        } else if cancel || !open {
            self.product_form = None;
        }
    }

    fn save_product_form(&mut self) {
        let Some(form) = self.product_form.clone() else {
            return;
        };
        let price = Money::parse(&form.price).unwrap_or(Money::ZERO);
        let tax_rate = TaxRate::ALL[form.tax_index];
        let barcode = if form.barcode.trim().is_empty() {
            None
        } else {
            Some(form.barcode.trim().to_string())
        };

        let result = match form.id {
            Some(id) => {
                let mut p = match self.db.get_product(id) {
                    Ok(Some(p)) => p,
                    Ok(None) => {
                        self.toast_err(format!("product {id} not found"));
                        return;
                    }
                    Err(e) => {
                        self.store_error(e);
                        return;
                    }
                };
                p.sku = form.sku.clone();
                p.barcode = barcode;
                p.name = form.name.clone();
                p.name_ar = form.name_ar.clone();
                p.category_id = form.category_id;
                p.price = price;
                p.tax_rate = tax_rate;
                p.unit = form.unit.clone();
                p.is_active = form.is_active;
                self.db.update_product(&p).map(|_| ())
            }
            None => self
                .db
                .create_product(&NewProduct {
                    sku: form.sku.clone(),
                    barcode,
                    name: form.name.clone(),
                    name_ar: form.name_ar.clone(),
                    category_id: form.category_id,
                    price,
                    tax_rate,
                    unit: form.unit.clone(),
                })
                .map(|_| ()),
        };

        match result {
            Ok(()) => {
                self.product_form = None;
                self.reload_products();
                self.toast_ok(self.t(Key::Saved));
            }
            Err(e) => self.store_error(e),
        }
    }
}
