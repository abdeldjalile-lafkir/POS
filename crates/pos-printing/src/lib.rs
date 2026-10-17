//! `pos-printing` — receipt model, ESC/POS text-mode encoder and transports.
//!
//! Receipts are English-only (per the plan), so we print using the printer's
//! native text mode. The module layout keeps a raster path open for future
//! bilingual receipts.

pub mod escpos;
pub mod receipt;
pub mod text;
pub mod transport;

pub use receipt::{PaperWidth, ReceiptDoc, ReceiptLine, ReceiptTaxLine};
pub use text::render_text;
pub use transport::{PrintError, PrinterConfig, Transport};
