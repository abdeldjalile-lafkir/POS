// Typed wrappers over the Tauri command surface (`pos_api::Api`).
import { invoke } from "@tauri-apps/api/core";
import type * as T from "./types";

export const api = {
  // auth
  hasUsers: () => invoke<boolean>("api_has_users"),
  bootstrapAdmin: (username: string, pin: string) =>
    invoke<T.User>("api_bootstrap_admin", { username, pin }),
  login: (username: string, pin: string) =>
    invoke<T.User>("api_login", { username, pin }),
  logout: () => invoke<void>("api_logout"),
  currentUser: () => invoke<T.User | null>("api_current_user"),
  loginUsers: () => invoke<T.User[]>("api_login_users"),
  listUsers: () => invoke<T.User[]>("api_list_users"),
  createUser: (username: string, pin: string, role: T.Role) =>
    invoke<number>("api_create_user", { username, pin, role }),
  setUserActive: (id: number, active: boolean) =>
    invoke<void>("api_set_user_active", { id, active }),
  resetPin: (id: number, pin: string) =>
    invoke<void>("api_reset_pin", { id, pin }),

  // catalogue
  listCategories: () => invoke<T.Category[]>("api_list_categories"),
  createCategory: (name: string, nameAr: string) =>
    invoke<number>("api_create_category", { name, nameAr }),
  listProducts: (includeInactive: boolean) =>
    invoke<T.Product[]>("api_list_products", { includeInactive }),
  searchProducts: (query: string, limit: number) =>
    invoke<T.Product[]>("api_search_products", { query, limit }),
  saveProduct: (input: T.ProductInput) =>
    invoke<number>("api_save_product", { input }),
  setProductActive: (id: number, active: boolean) =>
    invoke<void>("api_set_product_active", { id, active }),
  findByBarcode: (barcode: string) =>
    invoke<T.Product | null>("api_find_by_barcode", { barcode }),
  getProduct: (id: number) => invoke<T.Product | null>("api_get_product", { id }),
  importCatalogue: (csv: string) =>
    invoke<T.ImportSummary>("api_import_catalogue", { csv }),
  bundledCatalogue: () => invoke<string>("api_bundled_catalogue"),

  // inventory
  stockOverview: () => invoke<T.StockRow[]>("api_stock_overview"),
  lowStock: (threshold: number) =>
    invoke<T.StockRow[]>("api_low_stock", { threshold }),
  adjustStock: (input: T.AdjustStockInput) =>
    invoke<number>("api_adjust_stock", { input }),
  movementsFor: (productId: number) =>
    invoke<T.StockMovement[]>("api_movements_for", { productId }),

  // checkout
  createSale: (cart: T.CartInput, payment: T.PaymentInput) =>
    invoke<T.SaleResult>("api_create_sale", { cart, payment }),
  lastSale: () => invoke<T.SaleWithItems | null>("api_last_sale"),
  getSale: (id: number) => invoke<T.SaleWithItems | null>("api_get_sale", { id }),
  refundSale: (id: number) => invoke<T.SaleResult>("api_refund_sale", { id }),
  salesBetween: (from: number, to: number) =>
    invoke<T.Sale[]>("api_sales_between", { from, to }),

  // reports
  reports: (from: number, to: number) =>
    invoke<T.ReportsBundle>("api_reports", { from, to }),

  // settings
  settings: () => invoke<T.AppSettings>("api_settings"),
  saveStoreInfo: (info: T.StoreInfo) =>
    invoke<void>("api_save_store_info", { info }),
  savePrinter: (printer: T.PrinterSettings) =>
    invoke<void>("api_save_printer", { printer }),
  setLanguage: (lang: string) => invoke<void>("api_set_language", { lang }),
  backup: () => invoke<string>("api_backup"),

  // printing
  printReceipt: (id: number) => invoke<void>("api_print_receipt", { id }),
  printTest: () => invoke<void>("api_print_test"),
  receiptTextFor: (id: number) => invoke<string>("api_receipt_text_for", { id }),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
