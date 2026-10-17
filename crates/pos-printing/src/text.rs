//! A plain-text renderer, used for the on-screen preview and print-to-file.

use crate::receipt::ReceiptDoc;

/// Replace characters the ESC/POS code page cannot show with ASCII equivalents.
pub fn transliterate(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
            'ç' => 'c',
            'Ç' => 'C',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'ý' | 'ÿ' => 'y',
            '°' => 'o',
            '€' => 'E',
            '\u{2019}' | '\u{2018}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            c if c.is_ascii() => c,
            _ => '?',
        })
        .collect()
}

/// Right-align `right` on the same row as `left` within `cols` characters.
pub fn two_col(left: &str, right: &str, cols: usize) -> String {
    let left = transliterate(left);
    let right = transliterate(right);
    let lw = left.chars().count();
    let rw = right.chars().count();
    if lw + rw + 1 > cols {
        // Wrap: left line, then right-aligned value on the next line.
        let mut out = String::new();
        for chunk in wrap(&left, cols) {
            out.push_str(&chunk);
            out.push('\n');
        }
        out.push_str(&format!("{:>width$}", right, width = cols));
        return out;
    }
    let pad = cols - lw - rw;
    format!("{left}{}{right}", " ".repeat(pad))
}

fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= cols {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn center(text: &str, cols: usize) -> String {
    let t = transliterate(text);
    let w = t.chars().count();
    if w >= cols {
        return t;
    }
    let pad = (cols - w) / 2;
    format!("{}{}", " ".repeat(pad), t)
}

fn separator(cols: usize) -> String {
    "-".repeat(cols)
}

/// Render the receipt as plain text (no ESC/POS control codes).
pub fn render_text(doc: &ReceiptDoc) -> String {
    let cols = doc.paper_width.cols();
    let mut out = String::new();
    let mut push = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };

    push(center(
        if doc.store.name.is_empty() {
            "POS"
        } else {
            &doc.store.name
        },
        cols,
    ));
    if !doc.store.address.is_empty() {
        push(center(&doc.store.address, cols));
    }
    if !doc.store.phone.is_empty() {
        push(center(&format!("Tel: {}", doc.store.phone), cols));
    }
    if !doc.store.nif.is_empty() {
        push(center(&format!("NIF: {}", doc.store.nif), cols));
    }
    if !doc.store.nis.is_empty() {
        push(center(&format!("NIS: {}", doc.store.nis), cols));
    }
    if !doc.store.rc.is_empty() {
        push(center(&format!("RC: {}", doc.store.rc), cols));
    }
    if !doc.store.ai.is_empty() {
        push(center(&format!("AI: {}", doc.store.ai), cols));
    }
    push(separator(cols));
    push(center(doc.title, cols));
    push(two_col("Receipt", &doc.receipt_no, cols));
    push(two_col("Date", &doc.datetime, cols));
    push(two_col("Cashier", &doc.cashier, cols));
    push(separator(cols));

    for line in &doc.lines {
        push(transliterate(&line.name));
        let detail = format!("  {} x {}", line.qty, line.unit_price.format_amount());
        push(two_col(&detail, &line.total.format_dzd(), cols));
    }

    push(separator(cols));
    push(two_col("Subtotal (HT)", &doc.subtotal.format_dzd(), cols));
    if !doc.discount_total.is_zero() {
        push(two_col(
            "Discount",
            &format!("-{}", doc.discount_total.format_dzd()),
            cols,
        ));
    }
    for tl in &doc.tax_lines {
        push(two_col(
            &format!("TVA {}", tl.label),
            &tl.tax.format_dzd(),
            cols,
        ));
    }
    push(two_col("TOTAL", &doc.total.format_dzd(), cols));
    push(separator(cols));
    push(two_col(
        &format!("Paid ({})", doc.payment_method),
        &doc.tendered.format_dzd(),
        cols,
    ));
    push(two_col("Change", &doc.change.format_dzd(), cols));
    if doc.discount_total.is_zero() {
        // keep layout stable; nothing to add
    }
    push(separator(cols));
    if !doc.footer.is_empty() {
        push(center(&doc.footer, cols));
    }
    push(center("TVA incluse / Prices include TVA", cols));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_col_pads_to_width() {
        let row = two_col("Subtotal", "1 234,56 DA", 32);
        assert_eq!(row.chars().count(), 32);
        assert!(row.starts_with("Subtotal"));
        assert!(row.ends_with("1 234,56 DA"));
    }

    #[test]
    fn two_col_wraps_long_left() {
        let row = two_col("a very long product name indeed here", "9,00 DA", 20);
        assert!(row.contains('\n'));
        assert!(row.lines().all(|l| l.chars().count() <= 20));
    }

    #[test]
    fn transliterate_handles_french_accents() {
        assert_eq!(transliterate("Café Crème"), "Cafe Creme");
        assert_eq!(transliterate("Prix TTC"), "Prix TTC");
    }
}
