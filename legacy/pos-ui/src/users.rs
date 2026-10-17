//! Users screen: manage accounts, roles and PINs (admin only).

use eframe::egui;
use pos_domain::Role;
use pos_i18n::{t, Key};

use crate::util::hrow;
use crate::{PosApp, UserForm};

impl PosApp {
    pub fn users_screen(&mut self, ui: &mut egui::Ui) {
        let rtl = self.rtl();
        ui.add_space(6.0);
        hrow(ui, rtl, |ui| {
            ui.heading(self.t(Key::Users));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t(Key::AddUser)).clicked() {
                    self.user_form = Some(UserForm {
                        username: String::new(),
                        pin: String::new(),
                        role_admin: false,
                    });
                }
            });
        });
        ui.separator();

        let mut toggle: Option<(i64, bool)> = None;
        let mut reset: Option<i64> = None;

        egui::Grid::new("users_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                ui.strong(self.t(Key::Username));
                ui.strong(self.t(Key::Role));
                ui.strong(self.t(Key::Active));
                ui.strong("");
                ui.end_row();
                for u in &self.users {
                    ui.label(&u.username);
                    ui.label(if u.role.is_admin() {
                        self.t(Key::Admin)
                    } else {
                        self.t(Key::Cashier)
                    });
                    ui.label(if u.is_active { "✓" } else { "—" });
                    hrow(ui, rtl, |ui| {
                        if ui.small_button(self.t(Key::ResetPin)).clicked() {
                            reset = Some(u.id);
                        }
                        let label = if u.is_active {
                            self.t(Key::Deactivate)
                        } else {
                            self.t(Key::Activate)
                        };
                        if ui.small_button(label).clicked() {
                            toggle = Some((u.id, !u.is_active));
                        }
                    });
                    ui.end_row();
                }
            });

        if let Some((id, active)) = toggle {
            if let Err(e) = self.db.set_user_active(id, active) {
                self.store_error(e);
            }
            self.reload_users();
        }
        if let Some(id) = reset {
            self.pin_reset = Some((id, String::new()));
        }

        self.users_dialogs(ui.ctx());
    }

    pub fn users_dialogs(&mut self, ctx: &egui::Context) {
        self.user_form_dialog(ctx);
        self.pin_reset_dialog(ctx);
    }

    fn user_form_dialog(&mut self, ctx: &egui::Context) {
        if self.user_form.is_none() {
            return;
        }
        let lang = self.lang;
        let rtl = self.rtl();
        let mut open = true;
        let mut close = false;
        let mut save = false;

        {
            let form = self.user_form.as_mut().unwrap();
            egui::Window::new(t(lang, Key::AddUser))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("user_form").num_columns(2).show(ui, |ui| {
                        ui.label(t(lang, Key::Username));
                        ui.text_edit_singleline(&mut form.username);
                        ui.end_row();
                        ui.label(t(lang, Key::Pin));
                        ui.add(egui::TextEdit::singleline(&mut form.pin).password(true));
                        ui.end_row();
                        ui.label(t(lang, Key::Role));
                        hrow(ui, rtl, |ui| {
                            if ui
                                .selectable_label(!form.role_admin, t(lang, Key::Cashier))
                                .clicked()
                            {
                                form.role_admin = false;
                            }
                            if ui
                                .selectable_label(form.role_admin, t(lang, Key::Admin))
                                .clicked()
                            {
                                form.role_admin = true;
                            }
                        });
                        ui.end_row();
                    });
                    ui.add_space(8.0);
                    hrow(ui, rtl, |ui| {
                        if ui.button(t(lang, Key::Create)).clicked() {
                            save = true;
                        }
                        if ui.button(t(lang, Key::Cancel)).clicked() {
                            close = true;
                        }
                    });
                });
        }

        if save {
            if let Some(form) = self.user_form.clone() {
                let role = if form.role_admin {
                    Role::Admin
                } else {
                    Role::Cashier
                };
                match self.db.create_user(&form.username, &form.pin, role) {
                    Ok(_) => {
                        self.user_form = None;
                        self.reload_users();
                        self.toast_ok(self.t(Key::Saved));
                    }
                    Err(e) => self.store_error(e),
                }
            }
        } else if close || !open {
            self.user_form = None;
        }
    }

    fn pin_reset_dialog(&mut self, ctx: &egui::Context) {
        let Some((id, mut pin)) = self.pin_reset.clone() else {
            return;
        };
        let lang = self.lang;
        let rtl = self.rtl();
        let username = self
            .users
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.username.clone())
            .unwrap_or_default();
        let mut open = true;
        let mut close = false;
        let mut save = false;

        egui::Window::new(format!("{}: {username}", t(lang, Key::ResetPin)))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                hrow(ui, rtl, |ui| {
                    ui.label(t(lang, Key::NewPin));
                    ui.add(egui::TextEdit::singleline(&mut pin).password(true));
                });
                ui.add_space(8.0);
                hrow(ui, rtl, |ui| {
                    if ui.button(t(lang, Key::Save)).clicked() {
                        save = true;
                    }
                    if ui.button(t(lang, Key::Cancel)).clicked() {
                        close = true;
                    }
                });
            });

        if save {
            match self.db.set_user_pin(id, &pin) {
                Ok(()) => {
                    self.pin_reset = None;
                    self.toast_ok(self.t(Key::Saved));
                }
                Err(e) => {
                    self.pin_reset = Some((id, pin));
                    self.store_error(e);
                }
            }
        } else if close || !open {
            self.pin_reset = None;
        } else {
            // Persist the in-progress edit so typing survives the next frame.
            self.pin_reset = Some((id, pin));
        }
    }
}
