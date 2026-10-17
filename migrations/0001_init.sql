-- Migration 0001: initial schema.
-- Note: stock_movements carries user_id for accountability; a refunds audit table
-- is included so refunds never mutate the original sale row.

CREATE TABLE users (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    username    TEXT    NOT NULL UNIQUE,
    pin_hash    TEXT    NOT NULL,
    role        TEXT    NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);

CREATE TABLE categories (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT NOT NULL,
    name_ar  TEXT NOT NULL DEFAULT ''
);

CREATE TABLE products (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    sku           TEXT    NOT NULL DEFAULT '',
    barcode       TEXT    UNIQUE,
    name          TEXT    NOT NULL,
    name_ar       TEXT    NOT NULL DEFAULT '',
    category_id   INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    price_centimes INTEGER NOT NULL DEFAULT 0,
    tax_rate_bp   INTEGER NOT NULL DEFAULT 1900,
    unit          TEXT    NOT NULL DEFAULT 'pcs',
    is_active     INTEGER NOT NULL DEFAULT 1,
    image_path    TEXT
);
CREATE INDEX idx_products_name   ON products(name);
CREATE INDEX idx_products_active ON products(is_active);

CREATE TABLE stock_levels (
    product_id INTEGER PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    quantity   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE stock_movements (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id  INTEGER NOT NULL REFERENCES products(id),
    delta       INTEGER NOT NULL,
    reason      TEXT    NOT NULL,
    ref_sale_id INTEGER,
    note        TEXT,
    user_id     INTEGER,
    created_at  INTEGER NOT NULL
);
CREATE INDEX idx_stock_movements_product ON stock_movements(product_id);
CREATE INDEX idx_stock_movements_created ON stock_movements(created_at);

CREATE TABLE sales (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_no     TEXT    NOT NULL UNIQUE,
    created_at     INTEGER NOT NULL,
    cashier_id     INTEGER NOT NULL REFERENCES users(id),
    subtotal_c     INTEGER NOT NULL,
    tax_total_c    INTEGER NOT NULL,
    discount_c     INTEGER NOT NULL DEFAULT 0,
    total_c        INTEGER NOT NULL,
    payment_method TEXT    NOT NULL,
    tendered_c     INTEGER NOT NULL DEFAULT 0,
    change_c       INTEGER NOT NULL DEFAULT 0,
    status         TEXT    NOT NULL DEFAULT 'completed'
);
CREATE INDEX idx_sales_created ON sales(created_at);
CREATE INDEX idx_sales_cashier ON sales(cashier_id);

CREATE TABLE sale_items (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    sale_id        INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    product_id     INTEGER REFERENCES products(id),
    name_snapshot  TEXT    NOT NULL,
    unit_price_c   INTEGER NOT NULL,
    tax_rate_bp    INTEGER NOT NULL,
    qty            INTEGER NOT NULL,
    discount_c     INTEGER NOT NULL DEFAULT 0,
    line_subtotal_c INTEGER NOT NULL,
    line_tax_c     INTEGER NOT NULL,
    line_total_c   INTEGER NOT NULL
);
CREATE INDEX idx_sale_items_sale    ON sale_items(sale_id);
CREATE INDEX idx_sale_items_product ON sale_items(product_id);

CREATE TABLE refunds (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    sale_id    INTEGER NOT NULL REFERENCES sales(id),
    receipt_no TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    user_id    INTEGER NOT NULL,
    total_c    INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE counters (
    name  TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);
