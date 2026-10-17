//! Tauri command layer. Thin wrappers over the transport-agnostic `pos_api::Api`.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use pos_api::dto::{
    AdjustStockInput, AppSettings, CartInput, PaymentInput, PrinterSettings, ProductInput,
    ReportsBundle, SaleResult,
};
use pos_api::{Api, ImportSummary};
use pos_domain::{
    Category, Product, Sale, SaleWithItems, StockMovement, StoreInfo, User,
};
use pos_store::inventory::StockRow;
use pos_store::Db;
use tauri::{Manager, State};

/// Catalogue shipped with the app. Applied only when the database has no
/// products yet, so a fresh install starts with a populated catalogue.
const SEED_CATALOGUE: &str = include_str!("../../seed/catalogue.csv");

struct AppState {
    api: Mutex<Api>,
}

type R<T> = Result<T, String>;

fn lock<'a>(state: &'a State<'_, AppState>) -> MutexGuard<'a, Api> {
    state.api.lock().expect("api mutex poisoned")
}

fn resolve_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(d) = std::env::var("POS_DATA_DIR") {
        return PathBuf::from(d);
    }
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ---- auth -----------------------------------------------------------------

#[tauri::command]
fn api_has_users(state: State<'_, AppState>) -> R<bool> {
    lock(&state).has_users().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_bootstrap_admin(
    state: State<'_, AppState>,
    username: String,
    pin: String,
) -> R<User> {
    lock(&state)
        .bootstrap_admin(&username, &pin)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_login(state: State<'_, AppState>, username: String, pin: String) -> R<User> {
    lock(&state)
        .login(&username, &pin)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_logout(state: State<'_, AppState>) {
    lock(&state).logout();
}

#[tauri::command]
fn api_current_user(state: State<'_, AppState>) -> Option<User> {
    lock(&state).current_user().cloned()
}

#[tauri::command]
fn api_login_users(state: State<'_, AppState>) -> R<Vec<User>> {
    lock(&state).login_users().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_list_users(state: State<'_, AppState>) -> R<Vec<User>> {
    lock(&state).list_users().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_create_user(
    state: State<'_, AppState>,
    username: String,
    pin: String,
    role: String,
) -> R<i64> {
    lock(&state)
        .create_user(&username, &pin, &role)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_set_user_active(state: State<'_, AppState>, id: i64, active: bool) -> R<()> {
    lock(&state)
        .set_user_active(id, active)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_reset_pin(state: State<'_, AppState>, id: i64, pin: String) -> R<()> {
    lock(&state).reset_pin(id, &pin).map_err(|e| e.to_string())
}

// ---- catalogue ------------------------------------------------------------

#[tauri::command]
fn api_list_categories(state: State<'_, AppState>) -> R<Vec<Category>> {
    lock(&state).list_categories().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_create_category(
    state: State<'_, AppState>,
    name: String,
    name_ar: String,
) -> R<i64> {
    lock(&state)
        .create_category(&name, &name_ar)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_list_products(state: State<'_, AppState>, include_inactive: bool) -> R<Vec<Product>> {
    lock(&state)
        .list_products(include_inactive)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_search_products(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
) -> R<Vec<Product>> {
    lock(&state)
        .search_products(&query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_save_product(state: State<'_, AppState>, input: ProductInput) -> R<i64> {
    lock(&state).save_product(&input).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_set_product_active(state: State<'_, AppState>, id: i64, active: bool) -> R<()> {
    lock(&state)
        .set_product_active(id, active)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_find_by_barcode(
    state: State<'_, AppState>,
    barcode: String,
) -> R<Option<Product>> {
    lock(&state)
        .find_by_barcode(&barcode)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_get_product(state: State<'_, AppState>, id: i64) -> R<Option<Product>> {
    lock(&state).get_product(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_import_catalogue(
    state: State<'_, AppState>,
    csv: String,
) -> R<ImportSummary> {
    lock(&state)
        .import_catalogue_csv(&csv)
        .map_err(|e| e.to_string())
}

/// The CSV bundled with the app, so the UI can offer "reload bundled catalogue".
#[tauri::command]
fn api_bundled_catalogue() -> String {
    SEED_CATALOGUE.to_string()
}

/// Write the current catalogue to `<data_dir>/export/catalogue.csv` so it can be
/// copied into `apps/desktop/seed/` and shipped with the app.
#[tauri::command]
fn api_export_catalogue(state: State<'_, AppState>) -> R<String> {
    lock(&state).write_install_data().map_err(|e| e.to_string())
}

// ---- inventory ------------------------------------------------------------

#[tauri::command]
fn api_stock_overview(state: State<'_, AppState>) -> R<Vec<StockRow>> {
    lock(&state).stock_overview().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_low_stock(state: State<'_, AppState>, threshold: i64) -> R<Vec<StockRow>> {
    lock(&state).low_stock(threshold).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_adjust_stock(state: State<'_, AppState>, input: AdjustStockInput) -> R<i64> {
    lock(&state).adjust_stock(&input).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_movements_for(state: State<'_, AppState>, product_id: i64) -> R<Vec<StockMovement>> {
    lock(&state)
        .movements_for(product_id)
        .map_err(|e| e.to_string())
}

// ---- checkout -------------------------------------------------------------

#[tauri::command]
fn api_create_sale(
    state: State<'_, AppState>,
    cart: CartInput,
    payment: PaymentInput,
) -> R<SaleResult> {
    lock(&state)
        .create_sale(&cart, &payment)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_last_sale(state: State<'_, AppState>) -> R<Option<SaleWithItems>> {
    lock(&state).last_sale().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_get_sale(state: State<'_, AppState>, id: i64) -> R<Option<SaleWithItems>> {
    lock(&state).get_sale(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_refund_sale(state: State<'_, AppState>, id: i64) -> R<SaleResult> {
    lock(&state).refund_sale(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_sales_between(state: State<'_, AppState>, from: i64, to: i64) -> R<Vec<Sale>> {
    lock(&state)
        .sales_between(from, to)
        .map_err(|e| e.to_string())
}

// ---- reports --------------------------------------------------------------

#[tauri::command]
fn api_reports(state: State<'_, AppState>, from: i64, to: i64) -> R<ReportsBundle> {
    lock(&state).reports(from, to).map_err(|e| e.to_string())
}

// ---- settings -------------------------------------------------------------

#[tauri::command]
fn api_settings(state: State<'_, AppState>) -> R<AppSettings> {
    lock(&state).settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_save_store_info(state: State<'_, AppState>, info: StoreInfo) -> R<()> {
    lock(&state)
        .save_store_info(&info)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_save_printer(state: State<'_, AppState>, printer: PrinterSettings) -> R<()> {
    lock(&state)
        .save_printer(&printer)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn api_set_language(state: State<'_, AppState>, lang: String) -> R<()> {
    lock(&state).set_language(&lang).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_backup(state: State<'_, AppState>) -> R<String> {
    lock(&state).backup().map_err(|e| e.to_string())
}

// ---- printing -------------------------------------------------------------

#[tauri::command]
fn api_print_receipt(state: State<'_, AppState>, id: i64) -> R<()> {
    lock(&state).print_receipt(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn api_print_test(state: State<'_, AppState>) -> R<()> {
    lock(&state).print_test().map_err(|e| e.to_string())
}

#[tauri::command]
fn api_receipt_text_for(state: State<'_, AppState>, id: i64) -> R<String> {
    lock(&state)
        .receipt_text_for(id)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = resolve_dir(app.handle());
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let db = Db::open(dir.join("pos.db")).map_err(|e| e.to_string())?;
            let mut api = Api::new(db, dir);
            // Ship the app with a starter catalogue: applied only if empty.
            match api.seed_if_empty(SEED_CATALOGUE) {
                Ok(s) if s.total() > 0 => eprintln!(
                    "seeded catalogue: {} products, {} categories",
                    s.total(),
                    s.categories_added
                ),
                Ok(_) => {}
                Err(e) => eprintln!("catalogue seed failed: {e}"),
            }
            app.manage(AppState {
                api: Mutex::new(api),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_has_users,
            api_bootstrap_admin,
            api_login,
            api_logout,
            api_current_user,
            api_login_users,
            api_list_users,
            api_create_user,
            api_set_user_active,
            api_reset_pin,
            api_list_categories,
            api_create_category,
            api_list_products,
            api_search_products,
            api_save_product,
            api_set_product_active,
            api_find_by_barcode,
            api_get_product,
            api_import_catalogue,
            api_bundled_catalogue,
            api_export_catalogue,
            api_stock_overview,
            api_low_stock,
            api_adjust_stock,
            api_movements_for,
            api_create_sale,
            api_last_sale,
            api_get_sale,
            api_refund_sale,
            api_sales_between,
            api_reports,
            api_settings,
            api_save_store_info,
            api_save_printer,
            api_set_language,
            api_backup,
            api_print_receipt,
            api_print_test,
            api_receipt_text_for,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
