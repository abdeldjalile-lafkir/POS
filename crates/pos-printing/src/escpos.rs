//! ESC/POS byte encoding (text mode).
//!
//! English-only receipts mean we can use the printer's native text mode instead
//! of raster images. Non-ASCII characters are transliterated to the code page.

use crate::receipt::ReceiptDoc;
use crate::text::{transliterate, two_col};

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

struct Buf {
    bytes: Vec<u8>,
}

impl Buf {
    fn new() -> Self {
        Buf { bytes: Vec::new() }
    }

    fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.bytes.extend_from_slice(bytes);
        self
    }

    /// Initialize the printer.
    fn init(&mut self) -> &mut Self {
        self.raw(&[ESC, b'@'])
    }

    /// Select code page PC850 (French accents).
    fn code_page(&mut self) -> &mut Self {
        self.raw(&[ESC, b't', 2])
    }

    fn align(&mut self, n: u8) -> &mut Self {
        // 0 = left, 1 = center, 2 = right
        self.raw(&[ESC, b'a', n])
    }

    fn bold(&mut self, on: bool) -> &mut Self {
        self.raw(&[ESC, b'E', if on { 1 } else { 0 }])
    }

    fn double_size(&mut self, on: bool) -> &mut Self {
        // GS ! n : 0x11 = double width+height, 0x00 = normal
        self.raw(&[GS, b'!', if on { 0x11 } else { 0x00 }])
    }

    fn line(&mut self, text: &str) -> &mut Self {
        self.raw(transliterate(text).as_bytes());
        self.raw(b"\n")
    }

    fn cut(&mut self) -> &mut Self {
        // GS V 0 : full cut
        self.raw(&[GS, b'V', 0])
    }

    fn kick_drawer(&mut self) -> &mut Self {
        // ESC p m t1 t2
        self.raw(&[ESC, b'p', 0, 25, 250])
    }
}

fn separator(cols: usize) -> String {
    "-".repeat(cols)
}

/// Encode a receipt as ESC/POS bytes.
pub fn encode(doc: &ReceiptDoc, auto_cut: bool, drawer_kick: bool) -> Vec<u8> {
    let cols = doc.paper_width.cols();
    let mut b = Buf::new();
    b.init().code_page();

    // Header block, centered.
    b.align(1);
    b.double_size(true);
    b.line(if doc.store.name.is_empty() {
        "POS"
    } else {
        &doc.store.name
    });
    b.double_size(false);
    for field in [
        &doc.store.address,
        &doc.store.phone,
        &doc.store.nif,
        &doc.store.nis,
        &doc.store.rc,
        &doc.store.ai,
    ] {
        if !field.is_empty() {
            b.line(field);
        }
    }
    b.align(0);
    b.line(&separator(cols));
    b.align(1);
    b.bold(true).line(doc.title).bold(false);
    b.align(0);
    b.line(&two_col("Receipt", &doc.receipt_no, cols));
    b.line(&two_col("Date", &doc.datetime, cols));
    b.line(&two_col("Cashier", &doc.cashier, cols));
    b.line(&separator(cols));

    for line in &doc.lines {
        b.line(&transliterate(&line.name));
        let detail = format!("  {} x {}", line.qty, line.unit_price.format_amount());
        b.line(&two_col(&detail, &line.total.format_dzd(), cols));
    }

    b.line(&separator(cols));
    b.line(&two_col("Subtotal (HT)", &doc.subtotal.format_dzd(), cols));
    if !doc.discount_total.is_zero() {
        b.line(&two_col(
            "Discount",
            &format!("-{}", doc.discount_total.format_dzd()),
            cols,
        ));
    }
    for tl in &doc.tax_lines {
        b.line(&two_col(
            &format!("TVA {}", tl.label),
            &tl.tax.format_dzd(),
            cols,
        ));
    }
    b.bold(true);
    b.line(&two_col("TOTAL", &doc.total.format_dzd(), cols));
    b.bold(false);
    b.line(&separator(cols));
    b.line(&two_col(
        &format!("Paid ({})", doc.payment_method),
        &doc.tendered.format_dzd(),
        cols,
    ));
    b.line(&two_col("Change", &doc.change.format_dzd(), cols));
    b.line(&separator(cols));

    if !doc.footer.is_empty() {
        b.align(1).line(&doc.footer).align(0);
    }

    // Feed before cutting.
    b.raw(b"\n\n\n\n");
    if drawer_kick {
        b.kick_drawer();
    }
    if auto_cut {
        b.cut();
    }
    b.bytes
}

/// A short test ticket used by the Settings screen.
pub fn encode_test_ticket(store_name: &str, cols: usize, auto_cut: bool) -> Vec<u8> {
    let mut b = Buf::new();
    b.init().code_page().align(1);
    b.double_size(true);
    b.line(if store_name.is_empty() {
        "POS"
    } else {
        store_name
    });
    b.double_size(false);
    b.line("Printer test");
    b.align(0);
    b.line(&separator(cols));
    b.line(&two_col("TVA 19%", "19,00 DA", cols));
    b.line(&two_col("TOTAL", "119,00 DA", cols));
    b.line(&separator(cols));
    b.line("If you can read this, printing works.");
    b.raw(b"\n\n\n");
    if auto_cut {
        b.cut();
    }
    b.bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_has_init_and_text() {
        let bytes = encode_test_ticket("Boutique", 32, true);
        assert_eq!(&bytes[0..2], &[ESC, b'@']);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("Printer test"));
        assert!(text.contains("119,00 DA"));
        // ends with a cut
        assert!(bytes.ends_with(&[GS, b'V', 0]));
    }
}
