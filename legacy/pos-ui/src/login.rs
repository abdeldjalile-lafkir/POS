//! First-run bootstrap and the login screen (touch-friendly PIN pad).

use eframe::egui;
use pos_domain::Role;
use pos_i18n::Key;

use crate::PosApp;

impl PosApp {
    /// If there are no users at all, force the admin-creation screen.
    pub fn first_run_screen(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.heading(self.t(Key::FirstRunTitle));
                ui.label(self.t(Key::CreateAdmin));
                ui.add_space(16.0);
            });

            let width = 360.0;
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Grid::new("fr").num_columns(2).show(ui, |ui| {
                        ui.label(self.t(Key::Username));
                        ui.text_edit_singleline(&mut self.fr_username);
                        ui.end_row();
                        ui.label(self.t(Key::Pin));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fr_pin)
                                .password(true)
                                .desired_width(160.0),
                        );
                        ui.end_row();
                        ui.label(self.t(Key::ConfirmPin));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fr_pin2)
                                .password(true)
                                .desired_width(160.0),
                        );
                        ui.end_row();
                    });

                    if let Some(err) = &self.fr_error {
                        ui.colored_label(egui::Color32::from_rgb(180, 40, 40), err);
                    }

                    ui.add_space(8.0);
                    if ui.button(self.t(Key::Create)).clicked() {
                        self.finish_first_run();
                    }
                    ui.add_space(8.0);
                    ui.label(self.t(Key::ChoosePin));
                },
            );
        });
    }

    fn finish_first_run(&mut self) {
        let username = self.fr_username.trim().to_string();
        if username.is_empty() {
            self.fr_error = Some(self.t(Key::Required).to_string());
            return;
        }
        if self.fr_pin != self.fr_pin2 {
            self.fr_error = Some(self.t(Key::PinMismatch).to_string());
            return;
        }
        match self.db.create_user(&username, &self.fr_pin, Role::Admin) {
            Ok(id) => {
                self.has_users = true;
                self.session = self.db.get_user(id).ok().flatten();
                self.fr_pin.clear();
                self.fr_pin2.clear();
                self.fr_error = None;
                self.reload_users();
                self.toast_ok(self.t(Key::Saved));
            }
            Err(e) => self.fr_error = Some(e.to_string()),
        }
    }

    pub fn login_screen(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.heading(self.t(Key::AppTitle));
                ui.add_space(8.0);

                ui.label(self.t(Key::Username));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login_username).desired_width(180.0),
                    );
                });

                ui.add_space(6.0);
                ui.label(self.t(Key::Pin));
                let masked: String = "*".repeat(self.login_pin.chars().count());
                ui.label(egui::RichText::new(&masked).size(22.0));

                if let Some(err) = &self.login_error {
                    ui.colored_label(egui::Color32::from_rgb(180, 40, 40), err);
                }

                self.pin_pad(ui);

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for u in self.users.clone() {
                        if ui
                            .selectable_label(self.login_username == u.username, &u.username)
                            .clicked()
                        {
                            self.login_username = u.username.clone();
                            self.login_pin.clear();
                        }
                    }
                });
            });
        });
    }

    fn pin_pad(&mut self, ui: &mut egui::Ui) {
        let btns = [
            ["1", "2", "3"],
            ["4", "5", "6"],
            ["7", "8", "9"],
            ["", "0", ""],
        ];
        let mut submit = false;
        for row in btns {
            ui.horizontal(|ui| {
                // Center the pad.
                ui.add_space(120.0);
                for label in row {
                    if label.is_empty() {
                        ui.add_space(52.0);
                        continue;
                    }
                    let b = egui::Button::new(egui::RichText::new(label).size(20.0))
                        .min_size(egui::vec2(48.0, 44.0));
                    if ui.add(b).clicked()
                        && self.login_pin.chars().count() < 12 {
                            self.login_pin.push_str(label);
                        }
                }
            });
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(120.0);
            if ui
                .add(egui::Button::new(self.t(Key::Clear)).min_size(egui::vec2(150.0, 36.0)))
                .clicked()
            {
                self.login_pin.clear();
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(120.0);
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(self.t(Key::Enter)).size(18.0))
                        .min_size(egui::vec2(150.0, 40.0)),
                )
                .clicked()
            {
                submit = true;
            }
        });
        if submit {
            self.try_login();
        }
    }

    fn try_login(&mut self) {
        let username = self.login_username.trim().to_string();
        match self.db.authenticate(&username, &self.login_pin) {
            Ok(Some(user)) => {
                self.session = Some(user);
                self.login_pin.clear();
                self.login_error = None;
                self.login_attempts = 0;
                self.screen = crate::Screen::Register;
                self.reload_products();
            }
            Ok(None) => {
                self.login_attempts += 1;
                self.login_pin.clear();
                let base = self.t(Key::WrongCredentials).to_string();
                let left = 5u32.saturating_sub(self.login_attempts);
                self.login_error = Some(if left == 0 {
                    self.t(Key::TooManyAttempts).to_string()
                } else {
                    format!("{base} — {left} {}", self.t(Key::AttemptsLeft))
                });
            }
            Err(e) => self.login_error = Some(e.to_string()),
        }
    }
}
