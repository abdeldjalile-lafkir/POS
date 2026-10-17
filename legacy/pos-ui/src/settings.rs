//! Settings screen: store legal info, printer, language and data backup.

use eframe::egui;
use pos_i18n::Key;
use pos_store::settings as sk;

use crate::util::hrow;
use crate::PosApp;

impl PosApp {
    pub fn settings_screen(&mut self, ui: &mut egui::Ui) {
        let rtl = self.rtl();
        ui.add_space(6.0);
        ui.heading(self.t(Key::NavSettings));
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ---- store info ----
            ui.strong(self.t(Key::StoreInfo));
            egui::Grid::new("store_info").num_columns(2).show(ui, |ui| {
                ui.label(self.t(Key::StoreName));
                ui.text_edit_singleline(&mut self.store_info.name);
                ui.end_row();
                ui.label(self.t(Key::Address));
                ui.text_edit_singleline(&mut self.store_info.address);
                ui.end_row();
                ui.label(self.t(Key::Nif));
                ui.text_edit_singleline(&mut self.store_info.nif);
                ui.end_row();
                ui.label(self.t(Key::Nis));
                ui.text_edit_singleline(&mut self.store_info.nis);
                ui.end_row();
                ui.label(self.t(Key::Rc));
                ui.text_edit_singleline(&mut self.store_info.rc);
                ui.end_row();
                ui.label(self.t(Key::Ai));
                ui.text_edit_singleline(&mut self.store_info.ai);
                ui.end_row();
                ui.label(self.t(Key::Phone));
                ui.text_edit_singleline(&mut self.store_info.phone);
                ui.end_row();
                ui.label(self.t(Key::ReceiptFooter));
                ui.text_edit_singleline(&mut self.store_info.receipt_footer);
                ui.end_row();
            });
            if ui.button(self.t(Key::Save)).clicked() {
                match self.db.save_store_info(&self.store_info) {
                    Ok(()) => self.toast_ok(self.t(Key::Saved)),
                    Err(e) => self.store_error(e),
                }
            }

            ui.add_space(16.0);
            ui.separator();
            // ---- printer ----
            ui.strong(self.t(Key::Printer));
            hrow(ui, rtl, |ui| {
                ui.label(self.t(Key::Transport));
                for (idx, key) in [
                    (0usize, Key::TransportPreview),
                    (1, Key::TransportDevice),
                    (2, Key::TransportNetwork),
                ] {
                    if ui
                        .selectable_label(self.cfg_transport == idx, self.t(key))
                        .clicked()
                    {
                        self.cfg_transport = idx;
                    }
                }
            });

            egui::Grid::new("printer").num_columns(2).show(ui, |ui| {
                if self.cfg_transport == 2 {
                    ui.label(self.t(Key::Host));
                    ui.text_edit_singleline(&mut self.cfg_host);
                    ui.end_row();
                    ui.label(self.t(Key::Port));
                    ui.text_edit_singleline(&mut self.cfg_port);
                    ui.end_row();
                }
                if self.cfg_transport == 1 {
                    ui.label(self.t(Key::DevicePath));
                    ui.text_edit_singleline(&mut self.cfg_device);
                    ui.end_row();
                }
                ui.label(self.t(Key::PaperWidth));
                hrow(ui, rtl, |ui| {
                    for (mm, label) in [(58u32, "58 mm"), (80, "80 mm")] {
                        if ui
                            .selectable_label(self.cfg_paper_mm == mm, label)
                            .clicked()
                        {
                            self.cfg_paper_mm = mm;
                        }
                    }
                });
                ui.end_row();
                ui.label(self.t(Key::AutoCut));
                ui.checkbox(&mut self.cfg_auto_cut, "");
                ui.end_row();
                ui.label(self.t(Key::CashDrawer));
                ui.checkbox(&mut self.cfg_drawer, "");
                ui.end_row();
            });

            hrow(ui, rtl, |ui| {
                if ui.button(self.t(Key::Save)).clicked() {
                    self.save_printer_settings();
                }
                if ui.button(self.t(Key::TestPrint)).clicked() {
                    match self.printer_config().print_test(&self.store_info.name) {
                        Ok(()) => self.toast_ok(self.t(Key::Saved)),
                        Err(e) => self.toast_err(format!("{}: {e}", self.t(Key::PrintFailed))),
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            // ---- environment ----
            ui.strong(self.t(Key::Environment));
            hrow(ui, rtl, |ui| {
                ui.label(self.t(Key::Language));
                for l in pos_i18n::Lang::ALL {
                    if ui
                        .selectable_label(self.lang == l, l.native_name())
                        .clicked()
                    {
                        self.set_lang(l);
                    }
                }
            });
            hrow(ui, rtl, |ui| {
                ui.label(self.t(Key::ScanTiming));
                let mut gap = self.cfg_scan_gap.parse::<u64>().unwrap_or(self.scan_gap_ms);
                if ui
                    .add(egui::DragValue::new(&mut gap).range(5..=500))
                    .changed()
                {
                    self.cfg_scan_gap = gap.to_string();
                    self.scan_gap_ms = gap;
                }
            });
            hrow(ui, rtl, |ui| {
                if ui.button(self.t(Key::BackupDatabase)).clicked() {
                    self.do_backup();
                }
            });
        });
    }

    fn save_printer_settings(&mut self) {
        let transport = match self.cfg_transport {
            1 => "file",
            2 => "network",
            _ => "preview",
        };
        let ops: Vec<(&str, String)> = vec![
            (sk::KEY_PRINTER_TRANSPORT, transport.to_string()),
            (sk::KEY_PRINTER_HOST, self.cfg_host.clone()),
            (sk::KEY_PRINTER_PORT, self.cfg_port.clone()),
            (sk::KEY_PRINTER_DEVICE, self.cfg_device.clone()),
            (sk::KEY_PRINTER_PAPER_WIDTH, self.cfg_paper_mm.to_string()),
            (
                sk::KEY_PRINTER_AUTO_CUT,
                if self.cfg_auto_cut { "1" } else { "0" }.to_string(),
            ),
            (
                sk::KEY_PRINTER_DRAWER_KICK,
                if self.cfg_drawer { "1" } else { "0" }.to_string(),
            ),
            (sk::KEY_SCAN_MAX_GAP_MS, self.scan_gap_ms.to_string()),
        ];
        for (k, v) in ops {
            if let Err(e) = self.db.set_setting(k, &v) {
                self.store_error(e);
                return;
            }
        }
        self.toast_ok(self.t(Key::Saved));
    }

    fn do_backup(&mut self) {
        let stamp = pos_domain::now_unix();
        let path = self.data_dir.join(format!("backup-{stamp}.db"));
        match self.db.backup_to(&path) {
            Ok(()) => {
                let msg = format!("{}: {}", self.t(Key::BackupDone), path.display());
                self.toast_ok(msg);
            }
            Err(e) => self.store_error(e),
        }
    }

    /// Settings uses inline controls only; kept for symmetry with the other screens.
    pub fn settings_dialogs(&mut self, _ctx: &egui::Context) {}
}
