//! `pos-api` — the transport-agnostic service layer for the web UI.
//!
//! Every operation the React frontend needs lives here as a plain Rust method,
//! so it can be unit-tested without a GUI or a webview. The Tauri shell and the
//! (optional) HTTP server are thin wrappers over [`Api`].

pub mod dto;
pub mod import;

use std::path::PathBuf;

use pos_domain::{Cart, CartLine, Discount, Money, TaxRate, User};
use pos_printing::{PaperWidth, PrinterConfig, Transport};
use pos_store::settings as sk;
use pos_store::{Db, StoreError};

pub use dto::*;
pub use import::ImportSummary;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("{0}")]
    Message(String),
    #[error("not authenticated")]
    NotAuthenticated,
    #[error("administrator role required")]
    Forbidden,
    #[error("print error: {0}")]
    Print(#[from] pos_printing::PrintError),
}

pub type Result<T> = std::result::Result<T, ApiError>;

/// The application service. Holds the database, the data directory and the
/// current session.
pub struct Api {
    pub db: Db,
    pub data_dir: PathBuf,
    pub session: Option<User>,
}

impl Api {
    pub fn new(db: Db, data_dir: PathBuf) -> Self {
        Self {
            db,
            data_dir,
            session: None,
        }
    }

    pub fn current_user(&self) -> Option<&User> {
        self.session.as_ref()
    }

    fn user(&self) -> Result<&User> {
        self.session.as_ref().ok_or(ApiError::NotAuthenticated)
    }

    fn admin(&self) -> Result<&User> {
        let u = self.user()?;
        if u.role.is_admin() {
            Ok(u)
        } else {
            Err(ApiError::Forbidden)
        }
    }

    fn paper_width(&self) -> PaperWidth {
        let mm = self
            .db
            .get_setting_or(sk::KEY_PRINTER_PAPER_WIDTH, "80")
            .unwrap_or_default()
            .parse()
            .unwrap_or(80);
        PaperWidth::from_mm(mm)
    }

    /// The current printer configuration, assembled from settings.
    pub fn printer_config(&self) -> PrinterConfig {
        let transport = match self
            .db
            .get_setting_or(sk::KEY_PRINTER_TRANSPORT, "preview")
            .unwrap_or_default()
            .as_str()
        {
            "file" => Transport::File(
                self.db
                    .get_setting_or(sk::KEY_PRINTER_DEVICE, "/dev/usb/lp0")
                    .unwrap_or_default(),
            ),
            "network" => {
                let host = self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_HOST, "")
                    .unwrap_or_default();
                let port = self
                    .db
                    .get_setting_or(sk::KEY_PRINTER_PORT, "9100")
                    .unwrap_or_default();
                Transport::Network(format!("{}:{}", host.trim(), port.trim()))
            }
            _ => Transport::Preview,
        };
        PrinterConfig {
            transport,
            paper_width: self.paper_width(),
            auto_cut: self
                .db
                .get_setting_or(sk::KEY_PRINTER_AUTO_CUT, "1")
                .unwrap_or_default()
                != "0",
            drawer_kick: self
                .db
                .get_setting_or(sk::KEY_PRINTER_DRAWER_KICK, "1")
                .unwrap_or_default()
                != "0",
        }
    }

    fn build_cart(&self, input: &CartInput) -> Cart {
        let mut cart = Cart::new();
        for l in &input.lines {
            let mut line = CartLine::new(
                l.product_id,
                l.name.clone(),
                Money::from_centimes(l.unit_price),
                TaxRate::from_bp(l.tax_rate_bp),
            );
            line.qty = l.qty.max(1);
            if let Some(d) = &l.discount {
                if let Some(dd) = d.to_discount() {
                    line.discount = dd;
                }
            }
            cart.add_or_increment(line);
        }
        cart.sale_discount = input
            .sale_discount
            .as_ref()
            .and_then(|d| d.to_discount())
            .unwrap_or(Discount::None);
        cart
    }
}

mod service;
