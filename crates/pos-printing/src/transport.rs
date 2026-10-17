//! Transports: where ESC/POS bytes are sent.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::escpos;
use crate::receipt::{PaperWidth, ReceiptDoc};

#[derive(Debug, Error)]
pub enum PrintError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid network address: {0}")]
    Address(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PrintError>;

/// Where to send print data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum Transport {
    /// Do not send anywhere; the UI shows the preview only.
    #[default]
    Preview,
    /// A device node (e.g. `/dev/usb/lp0`) or a regular file for testing.
    File(String),
    /// A raw TCP printer, e.g. `192.168.1.50:9100`.
    Network(String),
}


impl Transport {
    pub fn send(&self, bytes: &[u8]) -> Result<()> {
        match self {
            Transport::Preview => Ok(()),
            Transport::File(path) => {
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(PrintError::Io)?;
                f.write_all(bytes)?;
                f.flush()?;
                Ok(())
            }
            Transport::Network(addr) => {
                use std::net::TcpStream;
                let stream =
                    TcpStream::connect_timeout(&parse_addr(addr)?, Duration::from_secs(5))?;
                stream.set_write_timeout(Some(Duration::from_secs(5)))?;
                let mut stream = stream;
                stream.write_all(bytes)?;
                stream.flush()?;
                Ok(())
            }
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Transport::Preview => "preview".into(),
            Transport::File(p) => format!("file:{p}"),
            Transport::Network(a) => format!("net:{a}"),
        }
    }
}

fn parse_addr(addr: &str) -> Result<std::net::SocketAddr> {
    use std::net::ToSocketAddrs;
    addr.to_socket_addrs()
        .map_err(|_| PrintError::Address(addr.to_string()))?
        .next()
        .ok_or_else(|| PrintError::Address(addr.to_string()))
}

/// Full printer configuration.
#[derive(Debug, Clone)]
pub struct PrinterConfig {
    pub transport: Transport,
    pub paper_width: PaperWidth,
    pub auto_cut: bool,
    pub drawer_kick: bool,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            transport: Transport::default(),
            paper_width: PaperWidth::Mm80,
            auto_cut: true,
            drawer_kick: true,
        }
    }
}

impl PrinterConfig {
    /// Render + send a receipt.
    pub fn print(&self, doc: &ReceiptDoc) -> Result<()> {
        let bytes = escpos::encode(doc, self.auto_cut, self.drawer_kick);
        self.transport.send(&bytes)
    }

    /// Render + send a test ticket.
    pub fn print_test(&self, store_name: &str) -> Result<()> {
        let bytes = escpos::encode_test_ticket(store_name, self.paper_width.cols(), self.auto_cut);
        self.transport.send(&bytes)
    }
}

// A small unused helper to keep the PathBuf import meaningful across builds.
#[allow(dead_code)]
fn _p() -> PathBuf {
    PathBuf::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::{PaperWidth, ReceiptDoc};
    use pos_domain::{Money, SaleStatus, SaleWithItems};
    use pos_domain::{PaymentMethod, Sale, StoreInfo};

    fn sample_doc() -> ReceiptDoc {
        let sale = Sale {
            id: 1,
            receipt_no: "20260101-0001".into(),
            created_at: 1_767_225_600,
            cashier_id: 1,
            cashier_name: "admin".into(),
            subtotal: Money::from_dzd(100),
            tax_total: Money::from_dzd(19),
            discount_total: Money::ZERO,
            total: Money::from_dzd(119),
            payment_method: PaymentMethod::Cash,
            tendered: Money::from_dzd(200),
            change: Money::from_dzd(81),
            status: SaleStatus::Completed,
        };
        let with = SaleWithItems {
            sale,
            items: vec![],
        };
        ReceiptDoc::from_sale(&with, &StoreInfo::default(), PaperWidth::Mm80)
    }

    #[test]
    fn file_transport_writes_bytes() {
        let dir = std::env::temp_dir().join(format!("pos-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("receipt.bin");
        let _ = std::fs::remove_file(&path);

        let cfg = PrinterConfig {
            transport: Transport::File(path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        cfg.print(&sample_doc()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], &[0x1B, b'@']);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn preview_transport_is_a_noop() {
        let cfg = PrinterConfig::default();
        assert!(cfg.print(&sample_doc()).is_ok());
        assert!(cfg.print_test("Store").is_ok());
    }
}
