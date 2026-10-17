//! Dev harness: launch the UI on a given screen with a logged-in user, for
//! screenshots and manual review. Not part of the shipped `pos` binary.
//!
//! Usage:
//!   POS_DATA_DIR=/tmp/posdb POS_DEV_USER=admin POS_DEV_PIN=1234 \
//!   POS_DEV_SCREEN=products cargo run -p pos-ui --example screens

use std::path::PathBuf;

use eframe::egui;
use pos_store::Db;
use pos_ui::{PosApp, Screen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::var("POS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/opencode/posdev"));
    std::fs::create_dir_all(&dir)?;
    let db = Db::open(dir.join("pos.db"))?;

    let user = std::env::var("POS_DEV_USER").unwrap_or_else(|_| "admin".into());
    let pin = std::env::var("POS_DEV_PIN").unwrap_or_else(|_| "1234".into());
    let screen = match std::env::var("POS_DEV_SCREEN").unwrap_or_default().as_str() {
        "products" => Screen::Products,
        "inventory" => Screen::Inventory,
        "reports" => Screen::Reports,
        "settings" => Screen::Settings,
        "users" => Screen::Users,
        _ => Screen::Register,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_title("POS (dev)"),
        ..Default::default()
    };

    eframe::run_native(
        "POS (dev)",
        options,
        Box::new(move |_cc| {
            let mut app = PosApp::new(db, dir);
            if let Ok(Some(u)) = app.db.authenticate(&user, &pin) {
                app.session = Some(u);
                app.screen = screen;
            } else {
                eprintln!("dev: could not authenticate {user}");
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
