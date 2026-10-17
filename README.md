# POS — Algerian cash-register app (Rust)

A single-binary desktop point-of-sale for a fixed cash-register terminal, built to
the spec in [`pos-rust-plan.md`](pos-rust-plan.md). Pure Rust, no webview, no
runtime database dependency.

**Status:** implemented and building. 44 tests pass; `cargo clippy` is clean.

## Features

- **Register / checkout** — scan or search, cart with quantity and per-line
  discount, sale-level discount (percent or amount), TVA breakdown by rate, cash /
  card / other payment, change calculation, receipt preview and print.
- **Barcode scanner** — USB HID keyboard-wedge with a global scan buffer and a
  timing heuristic (configurable max key gap). No drivers. A "simulate scan" box
  allows testing without hardware.
- **Products** — bilingual (Arabic/English) names, price (HT), TVA rate
  (19% / 9% / 0%), barcode (unique), SKU, category, unit, activate/deactivate.
- **Inventory** — cached stock levels plus an audited movement ledger
  (sale / purchase / adjustment / return, with user and note), low-stock filter.
- **Reports** — daily/weekly/monthly summaries, **TVA collected by rate**, top
  products, and a **Z-report** with cash-drawer reconciliation (cash expected =
  cash sales − refunds).
- **Users & auth** — argon2-hashed PINs, `Admin` / `Cashier` roles, first-run
  admin bootstrap, PIN pad, role-gated screens, PIN reset.
- **Printing** — ESC/POS text mode (English receipts), transports: preview,
  device/file (also `/dev/usb/lp0`), network TCP `host:9100`; auto-cut and
  cash-drawer kick. Print-to-file for testing without hardware.
- **Refunds** — full refund that restores stock, writes an audit row, and never
  deletes the original sale.
- **Localisation** — Arabic + English UI with an RTL toggle (mirrored layout).
- **Data safety** — SQLite WAL, embedded migrations, one-click database backup.

## Architecture

Cargo workspace (keeps domain logic pure and testable):

| Crate | Responsibility |
|---|---|
| `pos-domain` | Money (`i64` centimes), TVA, cart/total engine, entities. Pure, unit-tested. |
| `pos-store` | `rusqlite` (bundled) repos, embedded migrations, auth, reports. |
| `pos-printing` | Receipt model → ESC/POS text + plain-text, transports. |
| `pos-i18n` | Arabic/English string tables. |
| `pos-ui` | egui screens (login, register, products, inventory, reports, settings, users). |
| `pos-app` | `main.rs`, config, logging → the `pos` binary. |

Migrations live in `migrations/` and are embedded at compile time.

## Build & run

Requires a Rust toolchain and system GUI libraries (X11/Wayland).

```sh
cargo run --release          # launch the app
cargo test                   # run all tests
cargo clippy --all-targets   # lint
```

The database and logs live in `$XDG_DATA_HOME/pos` (default
`~/.local/share/pos`). Override with `POS_DATA_DIR` (directory) and `POS_DB` (file).

```sh
POS_DATA_DIR=/tmp/posdb cargo run --release
```

### Dev helper

Generate an argon2 PHC hash for a PIN (e.g. to seed a user manually):

```sh
cargo run -p pos-store --example hash_pin -- 1234
```

## Key design decisions

- **Money** is an `i64` centimes newtype; floats are never used. Amounts format
  as `1 234,56 DA`.
- **Tax model:** product prices are tax-exclusive (HT); TVA is added at checkout,
  computed **per line** and rounded half-up, then summed per rate for the receipt
  breakdown. Sale-level discounts are spread across lines proportionally using
  largest-remainder so bases stay exact.
- **Receipts are English-only**, which allows ESC/POS native text mode instead of
  raster images. The `pos-printing` layout keeps a raster path open for future
  bilingual receipts.
- **Arabic RTL** works natively: egui 0.37's text stack (harfrust + fontique)
  performs shaping/BiDi, and system fonts supply the glyphs.

## Known limitations / next steps

- Arabic RTL mirrors horizontal layouts and text runs, but individual widgets are
  not fully flipped; per-widget mirroring can be refined.
- Line-level discount editing uses a percentage drag; a richer discount editor
  (fixed amount per line, reason codes) is a follow-up.
- No opening-float / shift model yet, so the Z-report reconciles cash collected
  for a period rather than a cashier shift.
- Split payments are not modelled (single payment per sale).
- USB/serial printers are supported via their device file; a dedicated serial
  transport (baud configuration) is not implemented.
- Multi-terminal sync, multi-currency and bilingual receipts are out of scope
  for v1 (see the plan).

## Compliance notes (verify with your accountant)

- Receipt header carries store name, address, **NIF, NIS, RC** and an optional
  **AI** field.
- Receipt numbers are sequential per day and gap-free (allocated inside the sale
  transaction).
- The TVA rates modelled are 19% and 9% (plus 0%). Confirm whether **droit de
  timbre** (stamp duty) applies to your cash sales and whether receipts must be
  issued in Arabic — both would change the receipt/printing design.
