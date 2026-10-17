// TypeScript mirrors of the Rust DTOs. Money is always an integer number of
// centimes (1 DZD = 100 centimes). Tax rates are basis points (1900 = 19%).

export type Role = "admin" | "cashier";
export type PaymentMethod = "cash" | "card" | "other";
export type SaleStatus = "completed" | "refunded" | "voided";
export type StockReason = "sale" | "purchase" | "adjustment" | "return";

export interface User {
  id: number;
  username: string;
  role: Role;
  isActive: boolean;
  createdAt: number;
}

export interface Category {
  id: number;
  name: string;
  nameAr: string;
}

export interface Product {
  id: number;
  sku: string;
  barcode: string | null;
  name: string;
  nameAr: string;
  categoryId: number | null;
  /** Tax-exclusive price, in centimes. */
  price: number;
  /** Basis points: 1900 = 19%. */
  taxRate: number;
  unit: string;
  isActive: boolean;
  imagePath: string | null;
}

export interface StockRow {
  product: Product;
  quantity: number;
}

export interface StockMovement {
  id: number;
  productId: number;
  delta: number;
  reason: StockReason;
  refSaleId: number | null;
  note: string | null;
  userId: number | null;
  createdAt: number;
}

export interface SaleItem {
  id: number;
  saleId: number;
  productId: number | null;
  nameSnapshot: string;
  unitPrice: number;
  taxRate: number;
  qty: number;
  discount: number;
  lineSubtotal: number;
  lineTax: number;
  lineTotal: number;
}

export interface Sale {
  id: number;
  receiptNo: string;
  createdAt: number;
  cashierId: number;
  cashierName: string;
  subtotal: number;
  taxTotal: number;
  discountTotal: number;
  total: number;
  paymentMethod: PaymentMethod;
  tendered: number;
  change: number;
  status: SaleStatus;
}

export interface SaleWithItems {
  sale: Sale;
  items: SaleItem[];
}

export interface StoreInfo {
  name: string;
  address: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  phone: string;
  receiptFooter: string;
}

export interface PrinterSettings {
  transport: string;
  host: string;
  port: string;
  device: string;
  paperWidthMm: number;
  autoCut: boolean;
  drawerKick: boolean;
  scanMaxGapMs: number;
}

export interface AppSettings {
  store: StoreInfo;
  printer: PrinterSettings;
  language: string;
}

export interface RateAgg {
  rate: number;
  base: number;
  tax: number;
}

export interface DaySummary {
  day: string;
  count: number;
  gross: number;
  discount: number;
  taxTotal: number;
  total: number;
  cash: number;
  card: number;
}

export interface TopProduct {
  name: string;
  qty: number;
  total: number;
}

export interface ZReport {
  from: number;
  to: number;
  count: number;
  gross: number;
  discount: number;
  taxTotal: number;
  total: number;
  cashTotal: number;
  cardTotal: number;
  otherTotal: number;
  refundsTotal: number;
  cashExpected: number;
  byRate: RateAgg[];
}

export interface ReportsBundle {
  from: number;
  to: number;
  z: ZReport;
  top: TopProduct[];
  daily: DaySummary[];
}

export interface SaleResult {
  sale: Sale;
  items: SaleItem[];
  receiptText: string;
}

// ---- inputs ---------------------------------------------------------------

export interface DiscountInput {
  kind: "none" | "percent" | "amount";
  value: number;
}

export interface CartLineInput {
  productId: number | null;
  name: string;
  unitPrice: number;
  taxRateBp: number;
  qty: number;
  discount?: DiscountInput | null;
}

export interface CartInput {
  lines: CartLineInput[];
  saleDiscount?: DiscountInput | null;
}

export interface PaymentInput {
  method: PaymentMethod;
  tendered: number;
}

export interface ProductInput {
  id?: number | null;
  sku: string;
  barcode?: string | null;
  name: string;
  nameAr: string;
  categoryId?: number | null;
  price: number;
  taxRateBp: number;
  unit: string;
  isActive: boolean;
}

export interface AdjustStockInput {
  productId: number;
  delta: number;
  reason: StockReason;
  note?: string | null;
}
