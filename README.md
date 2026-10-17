# POS — Algerian cash-register app

A point-of-sale for a fixed cash-register terminal. **React + Tailwind + shadcn/ui
frontend in a Tauri v2 desktop shell**, backed by a pure-Rust core (SQLite,
exact money math, ESC/POS printing). Target market: Algeria — DZD, TVA, NIF/NIS/RC.

## Architecture

```
POS/
├─ crates/                    Rust core (no UI, fully tested)
│  ├─ pos-domain/   Money (i64 centimes), TVA, cart/total engine, entities
│  ├─ pos-store/    rusqlite (bundled) + embedded migrations, auth, reports
│  ├─ pos-printing/ ESC/POS text encoder + transports (preview/file/network)
│  └─ pos-api/      transport-agnostic service layer (the app's "backend API")
├─ apps/desktop/              Tauri v2 desktop app
│  ├─ seed/catalogue.csv      catalogue shipped with the app (embedded)
│  ├─ src/                    React + TypeScript + Tailwind v4 + shadcn/ui
│  └─ src-tauri/              thin `#[tauri::command]` wrappers over pos-api
├─ migrations/                SQL schema (embedded at compile time)
└─ legacy/                    the previous egui desktop app (reference only)
```

The UI never talks to SQLite or the printer directly. Every action is a
`pos-api` method exposed as a Tauri command; `pos-api` is unit-tested without a
webview, and the React app is a thin presentation layer over it.

## Prerequisites

- Rust toolchain (`rustup`)
- Node + **bun** (used for the frontend)
- Linux system libraries for Tauri v2:
  ```sh
  sudo pacman -S webkit2gtk-4.1
  ```
  (plus `gtk3`, `libsoup3`, `librsvg`, which are usually already present)

## Develop / build

```sh
cd apps/desktop
bun install
bun run tauri dev      # launches the desktop app
bun run tauri build    # produces the release bundle
```

Backend tests and lint (no webview needed):

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
```

The database and logs live in the OS app-data directory (`pos.db`), or set
`POS_DATA_DIR` to override (handy for development and testing).

## Features

- **Register** — scan or search, cart with quantity and per-line/sale discounts,
  live TVA breakdown by rate, cash/card/other payment, change, receipt preview
  and print. Scanning an unknown barcode opens a **quick-add dialog** (name +
  price, TVA defaulting to 0%) that creates the product and drops it in the cart;
  cashiers may create products, editing stays admin-only.
- **Ships with data** — a starter catalogue (`apps/desktop/seed/catalogue.csv`)
  is embedded in the binary and applied automatically on a fresh install; the
  same CSV can be pasted/imported later from Settings.
- **Products** — bilingual (Arabic/English) names, TVA rate (19% / 9% / 0%),
  barcode (unique), categories, activate/deactivate.
- **Inventory** — cached stock levels + audited movement ledger (sale/purchase/
  adjustment/return with user + note), low-stock filter.
- **Reports** — period summaries, **TVA collected by rate**, top products,
  **Z-report** with cash reconciliation, recent sales with refund + reprint.
- **Users & auth** — argon2 PINs, Admin/Cashier roles, first-run bootstrap,
  PIN pad login, PIN reset.
- **Settings** — store legal info (NIF/NIS/RC/AI), printer transport
  (preview/device/network), paper width, auto-cut, cash-drawer kick, test print,
  language (English/Arabic with RTL), database backup.
- **Printing** — ESC/POS text mode over network `host:9100`, a device node
  (`/dev/usb/lp0`), or a file; receipt is English-only per the spec.

## Design decisions

- **Money** is an `i64` centimes newtype; floats are never used. Display uses
  Algerian conventions (`1 234,56 DA`). The frontend mirrors these rules in
  `src/lib/totals.ts` so the cart shows the same TVA breakdown live.
- **Tax**: product prices are HT; TVA is computed per line, rounded half-up, and
  summed per rate. Sale-level discounts are spread across lines by
  largest-remainder so bases stay exact.
- **JSON** uses camelCase (`serde(rename_all = "camelCase")`).
- **Arabic RTL** is handled by the browser engine (right-to-left layout + text
  shaping), which is far more robust than the previous egui approach.
- **Font** — **Alexandria** (variable, covers Latin **and** Arabic) is bundled at
  `apps/desktop/public/fonts/Alexandria-Variable.ttf` under the SIL OFL
  (`OFL.txt` beside it). No runtime font download; it ships inside the app.

## Spec

See [`pos-rust-plan.md`](pos-rust-plan.md) for the original build plan.

## Shipping the app with your data

The catalogue the app ships with lives in **`apps/desktop/seed/catalogue.csv`**.
It is compiled into the binary and applied **only when the products table is
empty**, so a fresh install starts populated (against the normal app-data
directory, not `POS_DATA_DIR`).

Replace that file with your own products, then rebuild (`bun run tauri dev` /
`bun run tauri build`). Only the `name` column is required; everything else is
optional, and columns are matched by header so order doesn't matter.

| Column | Meaning |
|---|---|
| `barcode` | Unique barcode. **If it already exists, the row updates that product** (so re-importing is idempotent). |
| `name` | Product name (required). |
| `name_ar` | Arabic name. |
| `category` / `category_ar` | Category, created on demand. |
| `price` | Tax-exclusive price in DZD (`120.00` or `120,00`). |
| `vat` | TVA percent: `19`, `9`, or `0`. |
| `unit` | e.g. `pcs`, `kg`, `L` (default `pcs`). |
| `stock` | Optional absolute stock count; sets the level (adjustment is logged). |

To import later instead of rebuilding, use **Settings → Catalogue import**: paste
CSV into the box and press *Import CSV*, or click *Load bundled catalogue* to
re-apply the embedded seed. Quoted fields with commas are supported, and exports
from Excel/LibreOffice work as-is.

## Compliance notes (verify with your accountant)

- Receipt header carries store name, address, **NIF, NIS, RC** and optional **AI**.
- Receipt numbers are sequential per day and gap-free (allocated inside the sale
  transaction).
- Confirm whether **droit de timbre** (stamp duty) applies to cash sales and
  whether receipts must be issued in Arabic — either would change the design.
