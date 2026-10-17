//! `pos` — the single-binary desktop POS application.

use std::path::PathBuf;

use anyhow::{Context, Result};
use eframe::egui;
use pos_store::Db;
use pos_ui::PosApp;

/// Where the database and backups live.
fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("POS_DATA_DIR") {
        return PathBuf::from(d);
    }
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("pos")
}

fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::prelude::*;

    let file_appender = tracing_appender::rolling::daily(dir, "pos.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // Leak the guard so the writer stays alive for the whole process.
    std::mem::forget(_guard);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("pos=info,warn"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false);
    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
}

fn setup_style(ctx: &egui::Context) {
    // Touch-friendly defaults for a cash-register terminal.
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.interact_size.y = 28.0;
    });
}

fn main() -> Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).context("failed to create data directory")?;
    init_logging(&dir);

    let db_path = std::env::var("POS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dir.join("pos.db"));
    tracing::info!(path = %db_path.display(), "opening database");
    let db = Db::open(&db_path).context("failed to open database")?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("POS"),
        ..Default::default()
    };

    let app_dir = dir.clone();
    eframe::run_native(
        "POS",
        options,
        Box::new(move |cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(PosApp::new(db, app_dir)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe failed: {e}"))?;

    Ok(())
}
