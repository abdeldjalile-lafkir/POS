//! End-to-end test of the checkout pipeline: barcode lookup → cart → discount →
//! sale transaction → stock → receipt render/encode/print → refund → Z-report.

use pos_domain::SaleStatus;
use pos_domain::{
    Cart, CartLine, Discount, Money, PaymentMethod, Role, StockReason, StoreInfo, TaxRate,
};
use pos_printing::{PaperWidth, PrinterConfig, ReceiptDoc, Transport};
use pos_store::products::NewProduct;
use pos_store::sales::PaymentInfo;
use pos_store::Db;

#[test]
fn full_checkout_pipeline() {
    let mut db = Db::open_in_memory().unwrap();
    let admin = db.create_user("admin", "1234", Role::Admin).unwrap();

    let milk = db
        .create_product(&NewProduct {
            name: "Milk 1L".into(),
            barcode: Some("6130000000019".into()),
            price: Money::from_dzd(120),
            tax_rate: TaxRate::TVA_9,
            ..Default::default()
        })
        .unwrap();
    let coffee = db
        .create_product(&NewProduct {
            name: "Café".into(),
            price: Money::from_dzd(250),
            tax_rate: TaxRate::TVA_19,
            ..Default::default()
        })
        .unwrap();

    db.adjust_stock(milk, 100, StockReason::Purchase, None, Some(admin))
        .unwrap();
    db.adjust_stock(coffee, 100, StockReason::Purchase, None, Some(admin))
        .unwrap();

    // A scanned barcode resolves to a product.
    let scanned = db.find_by_barcode("6130000000019").unwrap().unwrap();
    assert_eq!(scanned.id, milk);

    // Build the cart the way the Register screen does.
    let mut cart = Cart::new();
    cart.add_or_increment(CartLine::new(
        Some(scanned.id),
        scanned.name.clone(),
        scanned.price,
        scanned.tax_rate,
    ));
    cart.add_or_increment(CartLine::new(
        Some(coffee),
        "Café",
        Money::from_dzd(250),
        TaxRate::TVA_19,
    ));
    cart.lines[1].qty = 2;
    cart.sale_discount = Discount::PercentBp(500); // 5%

    let totals = cart.totals();
    // Internal consistency: total == subtotal - sale discount + tax.
    assert_eq!(
        totals.total,
        totals.subtotal - totals.sale_discount + totals.tax_total
    );

    let sale = db
        .create_sale(
            &cart,
            admin,
            PaymentInfo {
                method: PaymentMethod::Cash,
                tendered: Money::from_dzd(2000),
            },
        )
        .unwrap();

    assert_eq!(sale.sale.total, totals.total);
    assert_eq!(sale.items.len(), 2);
    // Stock decremented by the quantities sold.
    assert_eq!(db.stock_for(milk).unwrap(), 99);
    assert_eq!(db.stock_for(coffee).unwrap(), 98);
    // Sum of line totals equals the receipt total.
    let line_sum: Money = sale.items.iter().map(|i| i.line_total).sum();
    assert_eq!(line_sum, sale.sale.total);

    // Receipt renders to text and encodes to ESC/POS, then prints to a file.
    let store = StoreInfo {
        name: "Superette Test".into(),
        nif: "000123456789012".into(),
        ..Default::default()
    };
    let doc = ReceiptDoc::from_sale(&sale, &store, PaperWidth::Mm80);
    let text = pos_printing::render_text(&doc);
    assert!(text.contains("Milk 1L"));
    assert!(text.contains("TOTAL"));
    assert!(text.contains("NIF: 000123456789012"));

    let dir = std::env::temp_dir().join(format!("pos-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("receipt.bin");
    let _ = std::fs::remove_file(&path);
    PrinterConfig {
        transport: Transport::File(path.to_string_lossy().into_owned()),
        ..Default::default()
    }
    .print(&doc)
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[0..2], &[0x1B, b'@']); // ESC @ init
    assert!(bytes.windows(3).any(|w| w == [0x1D, b'V', 0])); // cut
    let _ = std::fs::remove_file(&path);

    // Z-report before refund shows one completed sale.
    let now = pos_domain::now_unix();
    let z = db.z_report(now - 60, now + 60).unwrap();
    assert_eq!(z.count, 1);
    assert_eq!(z.total, totals.total);
    assert!(!z.by_rate.is_empty()); // both 19% and 9% appear

    // Refund restores stock and is recorded, but never deletes the sale.
    let refunded = db.refund_sale(sale.sale.id, admin).unwrap();
    assert_eq!(refunded.sale.status, SaleStatus::Refunded);
    assert_eq!(db.stock_for(milk).unwrap(), 100);
    assert_eq!(db.stock_for(coffee).unwrap(), 100);

    let z = db.z_report(now - 60, now + 60).unwrap();
    assert_eq!(z.count, 0);
    assert_eq!(z.refunds_total, totals.total);
    assert_eq!(z.cash_expected(), Money::ZERO - totals.total); // cash 0 − refunds
}
