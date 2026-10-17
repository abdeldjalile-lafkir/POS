# POS System in Rust — Build Plan

> Status: **FINALIZED (not yet implemented)**
> Target market: Algeria — DZD, TVA, NIF/NIS/RC
> Nothing has been installed, built, or executed. This document is the specification for the build phase.

---

## 1. Locked Decisions

| Area | Decision |
|---|---|
| GUI framework | **egui + eframe** — pure Rust, single native binary, no webview/runtime deps |
| Database | **rusqlite** with `bundled` feature (statically linked SQLite) + `rusqlite_migration` — keeps the single-binary promise |
| Money | `i64` **centimes** newtype. Never floats. Round half-up at total. |
| Currency | **DZD only**, formatted `1 234,56 DA` |
| Tax | Per-product TVA **19% / 9% / 0%** (basis points). Breakdown by rate printed on receipt. |
| Receipt language | **English only** |
| Users & auth | **From day one**: argon2-hashed PIN, roles `Admin` / `Cashier`, first-run admin bootstrap |
| Barcode scanner | **USB HID keyboard-wedge**, global scan buffer + timing detection + configurable prefix/suffix |
| Refunds | Simple **full-refund** (M6) |
| Discounts | Simple per-line & per-sale (amount or %), in v1 |
| UI language | **Arabic + English** with toggle (RTL mirrored when Arabic active) |
| Legal receipt fields | Store name, address, **NIF, NIS, RC** |
| Platform | Desktop app for a fixed cash-register terminal |

### Consequence of English-only receipts
ESC/POS thermal printers use fixed code pages and do **no Arabic letter shaping**. Because receipts are English-only, we can print via **ESC/POS native text mode** instead of raster images — simpler and faster. The `pos-printing` crate will still be structured so a raster renderer can be added later if bilingual receipts are ever wanted.

---

## 2. Tech Stack

| Concern | Crate / choice | Why |
|---|---|---|
| GUI | `eframe` / `egui` | Pure Rust, immediate-mode, one native binary |
| DB | `rusqlite` (`bundled`) | Statically linked SQLite, no external `.so` |
| Migrations | `rusqlite_migration` | Versioned, repeatable schema |
| Money | custom `i64` centimes newtype | Exact arithmetic |
| PIN hashing | `argon2` | Modern password/PIN hashing |
| Printing | `escpos-rs` (or `escpos`) | ESC/POS over network :9100 / USB / serial |
| Arabic shaping | `rustybuzz` + `unicode-bidi` | Contextual shaping + BiDi for Arabic UI |
| Serialization | `serde` + `ron`/`toml` | Config & settings |
| Errors | `thiserror` (libs) + `anyhow`/`color-eyre` (app) | Clean layering |
| Barcode | keyboard-wedge capture | No drivers needed |

**Alternative GUI options considered:** `iced` (Elm-style, more boilerplate), `Tauri` (webview — adds a dependency, but strongest native RTL support). Chosen: **egui**.

---

## 3. Project Structure

```
pos/
├─ Cargo.toml                     (workspace)
├─ crates/
│  ├─ pos-domain/    Money, tax, cart, sale totals, user roles        [pure, unit-tested]
│  ├─ pos-store/     rusqlite repos + migrations + entities
│  ├─ pos-printing/  receipt model → ESC/POS text, transports
│  ├─ pos-i18n/      ar/en strings + Arabic shaping/BiDi pipeline
│  ├─ pos-ui/        egui screens (login, register, products, inventory, reports, settings)
│  └─ pos-app/       main.rs, DB file, config, wiring → ONE binary
├─ migrations/*.sql
└─ assets/           Arabic (Noto Naskh) + Latin fonts, icons
```

If preferred, this can collapse into a single crate with modules. Default: workspace (keeps domain logic pure and testable).

---

## 4. Data Model (SQLite)

- **products** — id, sku, barcode (unique), name, name_ar, category_id, price_centimes, tax_rate_bp, unit, is_active, image_path
- **categories** — id, name, name_ar
- **stock_movements** — id, product_id, delta, reason (sale/purchase/adjustment/return), ref_sale_id, note, created_at  *(audit ledger)*
- **stock_levels** — product_id (PK), quantity  *(fast cached current stock, kept in sync with the ledger in one transaction)*
- **sales** — id, receipt_no, created_at, cashier_id, subtotal_c, tax_total_c, discount_c, total_c, payment_method, tendered_c, change_c, status (completed/refunded/voided)
- **sale_items** — id, sale_id, product_id, **name_snapshot**, **unit_price_snapshot**, **tax_rate_snapshot**, qty, line_subtotal_c, line_tax_c, line_total_c  *(snapshots so past receipts stay correct if a price changes)*
- **users** — id, username, pin_hash (argon2), role (`Admin`/`Cashier`), is_active, created_at
- **settings** — key/value: store name, address, NIF, NIS, RC, receipt footer, printer config, language

---

## 5. Screens (egui)

1. **Login** — touch-friendly numeric PIN pad; session holds `current_cashier_id`.
2. **Register / Checkout** — always-focused scan/search box, product results, cart with qty edit & line remove, discounts, totals panel (Subtotal · TVA · **Total DZD**), Pay → tendered/change dialog → receipt preview & print.
3. **Products** — searchable list, add/edit dialog (price, tax rate, barcode, bilingual name), activate/deactivate.
4. **Inventory** — current stock, low-stock filter, manual adjustment, movement history per product.
5. **Reports** — daily/monthly sales, **TVA collected by rate**, top products, **Z-report** for end-of-day cash reconciliation.
6. **Settings** — store legal info (NIF/NIS/RC), default tax rate, receipt header/footer, printer selection, language.

**Role gating:** Cashier → Register only. Admin → Products, Inventory, Reports, Settings, user management.

---

## 6. Auth Model

- `users` table with **argon2-hashed PIN**.
- **First-run bootstrap**: if no users exist, force "create admin" screen.
- Login via numeric PIN pad; session tracks `current_cashier_id`.
- Every `sales` row records `cashier_id`; logout / switch-user from the register.

---

## 7. Printing Module (`pos-printing`)

- `ReceiptDoc` (structured) → renderers: **on-screen preview**, **ESC/POS text**, plain-text/PDF fallback.
- Transports: **Network TCP :9100** (most common), **USB/Serial**, and **print-to-file** (for testing without hardware).
- Includes **cash-drawer kick** (`ESC p`) and auto-cut (`GS V`).
- Config in Settings: connection type, address/port, paper width (58/80mm).
- English-only v1 → text mode. Raster path reserved for future bilingual receipts.

---

## 8. Barcode Scanner

- HID keyboard-wedge devices type the barcode + suffix (usually `Enter`). No driver.
- **Global scan buffer** fed from egui input events — works even when no text field is focused.
- **Timing heuristic**: scanner types fast (< ~30 ms between keys) then sends suffix; distinguishes a scan from slow human typing. Threshold configurable.
- **Prefix/suffix configurable** in Settings; default suffix = Enter.
- Supported symbologies (decoded by the scanner itself): **EAN-13 / EAN-8 / UPC-A** (retail goods), **Code-128 / Code-39** (in-store labels / internal SKUs). Lookup by `products.barcode` (unique index).
- **Checkout behavior:** valid scan → increment qty if in cart else add; unknown barcode → prompt to create product or add as loose/weighted item; damaged barcode → manual search.
- **Testability:** a "simulate scan" input allows full testing without hardware.

---

## 9. Algerian Specifics

- Currency **DZD**, formatted `1 234,56 DA` (space thousands, comma decimal).
- **TVA 19%** default, **9%** reduced, **0%** exempt — selectable per product; receipt shows TVA breakdown by rate.
- Receipt header carries legally expected fields: store name, address, **NIF, NIS, RC**, date/time, receipt number, cashier, item codes.
- UI bilingual **Arabic / English** (per decision, RTL mirrored when Arabic active). Business data (product names) supports bilingual fields.

---

## 10. Milestones (build order)

- **M0 — Foundation:** install toolchain via `rustup` + system GUI libs; workspace scaffold; SQLite open + migrations; `Money`/tax core **with unit tests**; i18n string table skeleton. *(No UI — get build + tests green first.)*
- **M1 — Auth:** users table, argon2 PIN, first-run admin bootstrap, login PIN pad, roles, session.
- **M2 — Products & Inventory:** product CRUD (bilingual name, price, tax rate, barcode), categories, stock ledger + current levels, low-stock view, manual adjustments.
- **M3 — Checkout:** global scan buffer + search → cart → totals → **single transaction** (persist sale + decrement stock) → receipt preview; discounts.
- **M4 — Printing:** ESC/POS receipt renderer (text mode), transports (network/USB/serial/file), cash drawer, auto-cut, printer config.
- **M5 — Oversight:** reports (daily sales, **TVA by rate**, Z-report/cash reconciliation), Settings (store legal info, printer, language).
- **M6 — Polish & hardware:** full Arabic/RTL shaping across UI, real scanner config (prefix/suffix/timing), refunds.

---

## 11. Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Arabic shaping / RTL in egui | **High** | Isolate inside `pos-i18n`; fallbacks: render labels as textures, or reconsider Tauri (webview has native RTL) |
| ESC/POS Arabic (code pages) | High | Avoided — receipts are English-only (text mode) |
| No Rust installed yet | Low | M0 step 1: `rustup` + `libx11`/`wayland`/`libudev` dev packages |
| Printer hardware unknown | Medium | Support network/USB/serial + print-to-file so development isn't blocked |

---

## 12. Open / Deferred Items

- Printer model/connection (network 80mm, USB, other) — all transports configurable + testable without hardware in the meantime.
- Bilingual receipts (would require adding a raster renderer in `pos-printing`).
- Multi-currency / multi-terminal / client-server sync (explicitly out of scope for v1).

---

## 13. Immediate Next Step (when build begins)

Start at **M0**: `rustup` toolchain + system GUI libs → workspace scaffold → SQLite open + migrations → tested `Money`/tax core → i18n skeleton. No UI until the core compiles and tests pass.

> To begin implementation, switch the agent out of Plan mode to a build agent.
