// Dev-only mock of the Tauri IPC bridge, used to preview the UI in a browser
// without the native shell. Enabled by aliasing `@tauri-apps/api/core` to this
// module when VITE_MOCK=1 (see vite.config.ts). Never used in the real app.

type Args = Record<string, unknown> | undefined;

const now = Math.floor(Date.now() / 1000);
const user = {
  id: 1,
  username: "admin",
  role: "admin",
  isActive: true,
  createdAt: now,
};

const categories = [
  { id: 1, name: "Boissons", nameAr: "مشروبات" },
  { id: 2, name: "Épicerie", nameAr: "بقالة" },
  { id: 3, name: "Hygiène", nameAr: "نظافة" },
];

const products = [
  ["SKU001", "6130000000019", "Milk 1L", "حليب 1 لتر", 1, 12000, 900],
  ["SKU002", "6130000000026", "Bread", "خبز", 2, 1500, 0],
  ["SKU003", "6130000000033", "Mineral Water 1.5L", "ماء معدني", 1, 4000, 900],
  ["SKU004", "6130000000040", "Coffee 250g", "قهوة 250 غ", 2, 45000, 1900],
  ["SKU005", "6130000000057", "Sugar 1kg", "سكر 1 كغ", 2, 9000, 1900],
  ["SKU006", "6130000000064", "Olive Oil 1L", "زيت زيتون", 2, 78000, 1900],
  ["SKU007", "6130000000071", "Shampoo", "شامبو", 3, 32000, 1900],
  ["SKU008", "6130000000088", "Tomato Paste", "معجون طماطم", 2, 18000, 1900],
  ["SKU009", "6130000000095", "Yogurt", "ياغورت", 1, 2500, 900],
  ["SKU010", "6130000000101", "Detergent 2L", "منظف 2 لتر", 3, 55000, 1900],
].map((r, i) => ({
  id: i + 1,
  sku: r[0] as string,
  barcode: r[1] as string,
  name: r[2] as string,
  nameAr: r[3] as string,
  categoryId: r[4] as number,
  price: r[5] as number,
  taxRate: r[6] as number,
  unit: "pcs",
  isActive: i !== 9,
  imagePath: null,
}));

const stock = products.map((p, i) => ({ product: p, quantity: [42, 0, 120, 7, 35, 3, 18, 60, 9, 12][i] }));

const zReport = {
  from: now - 86400,
  to: now,
  count: 37,
  gross: 486500,
  discount: 12300,
  taxTotal: 74120,
  total: 548320,
  cashTotal: 312000,
  cardTotal: 236320,
  otherTotal: 0,
  refundsTotal: 11900,
  cashExpected: 300100,
  byRate: [
    { rate: 1900, base: 262000, tax: 49780 },
    { rate: 900, base: 168000, tax: 15120 },
    { rate: 0, base: 44320, tax: 0 },
  ],
};

const sales = Array.from({ length: 6 }).map((_, i) => ({
  id: 100 - i,
  receiptNo: `20261017-${String(37 - i).padStart(4, "0")}`,
  createdAt: now - i * 640,
  cashierId: 1,
  cashierName: i % 2 ? "sara" : "admin",
  subtotal: 45000 + i * 1200,
  taxTotal: 8550,
  discountTotal: i === 2 ? 1390 : 0,
  total: 53550 + i * 1200,
  paymentMethod: i % 3 === 0 ? "card" : "cash",
  tendered: 60000,
  change: 6450,
  status: i === 3 ? "refunded" : "completed",
}));

const settings = {
  store: {
    name: "Superette El Baraka",
    address: "12 Rue Didouche Mourad, Alger",
    nif: "000123456789012",
    nis: "000123456789013",
    rc: "16/00-1234567 B 12",
    ai: "16050123456",
    phone: "+213 21 00 00 00",
    receiptFooter: "Merci de votre visite",
  },
  printer: {
    transport: "preview",
    host: "192.168.0.50",
    port: "9100",
    device: "/dev/usb/lp0",
    paperWidthMm: 80,
    autoCut: true,
    drawerKick: true,
    scanMaxGapMs: 30,
  },
  language: (import.meta.env.VITE_MOCK_LANG as string) ?? "en",
};

const receiptText = `Superette El Baraka
NIF: 000123456789012  NIS: 000123456789013
RC: 16/00-1234567 B 12
--------------------------------
RECEIPT
Receipt           20261017-0037
Date        2026-10-17 13:20:11
Cashier                   admin
--------------------------------
Milk 1L
  2 x 120,00          240,00 DA
Coffee 250g
  1 x 450,00          450,00 DA
--------------------------------
Subtotal (HT)         689,35 DA
Discount              -34,47 DA
TVA 19%                79,00 DA
TOTAL                 733,88 DA
--------------------------------
Paid (Cash)           800,00 DA
Change                 66,12 DA
--------------------------------`;

function num(a: Args, k: string, d = 0): number {
  const v = a?.[k];
  return typeof v === "number" ? v : d;
}

export async function invoke<T>(cmd: string, args?: Args): Promise<T> {
  const a = args ?? {};

  // Commands that mutate the fixture catalogue.
  if (cmd === "api_save_product") {
    const input = a.input as Record<string, unknown>;
    const id = input.id as number | null | undefined;
    if (id) {
      const p = products.find((x) => x.id === id);
      if (p) Object.assign(p, input, { taxRate: input.taxRateBp });
      return id as T;
    }
    const nextId = Math.max(0, ...products.map((p) => p.id)) + 1;
    (products as unknown[]).push({
      id: nextId,
      sku: (input.sku as string) ?? "",
      barcode: (input.barcode as string) ?? null,
      name: input.name as string,
      nameAr: (input.nameAr as string) ?? "",
      categoryId: (input.categoryId as number) ?? null,
      price: input.price as number,
      taxRate: input.taxRateBp as number,
      unit: (input.unit as string) ?? "pcs",
      isActive: true,
      imagePath: null,
    });
    return nextId as T;
  }
  if (cmd === "api_get_product") {
    return (products.find((p) => p.id === a.id) ?? null) as T;
  }
  if (cmd === "api_import_catalogue") {
    return {
      productsAdded: 3,
      productsUpdated: 2,
      categoriesAdded: 1,
      stockAdjusted: 3,
      rowsSkipped: 0,
    } as T;
  }
  if (cmd === "api_bundled_catalogue") {
    return "barcode,name,price,vat,stock\n6130000000999,Demo item,99.00,0,10\n" as T;
  }

  const fixtures: Record<string, unknown> = {
    api_has_users: true,
    api_login_users: [user],
    api_current_user: user,
    api_login: user,
    api_bootstrap_admin: user,
    api_logout: null,
    api_list_users: [
      user,
      { id: 2, username: "sara", role: "cashier", isActive: true, createdAt: now },
      { id: 3, username: "karim", role: "cashier", isActive: false, createdAt: now },
    ],
    api_create_user: 4,
    api_set_user_active: null,
    api_reset_pin: null,
    api_list_categories: categories,
    api_create_category: 4,
    api_list_products: products,
    api_search_products: products.filter((p) => {
      const q = String(a.query ?? "").toLowerCase();
      return (
        !q ||
        p.name.toLowerCase().includes(q) ||
        p.sku.toLowerCase().includes(q) ||
        (p.barcode ?? "").includes(q)
      );
    }),
    api_save_product: 1,
    api_set_product_active: null,
    api_find_by_barcode: (productId => (productId ? productId : null))(
      products.find((p) => p.barcode === a.barcode),
    ),
    api_stock_overview: stock,
    api_low_stock: stock.filter((s) => s.quantity <= num(a, "threshold", 5)),
    api_adjust_stock: 42,
    api_movements_for: [
      { id: 1, productId: 1, delta: 50, reason: "purchase", refSaleId: null, note: "initial", userId: 1, createdAt: now - 7200 },
      { id: 2, productId: 1, delta: -8, reason: "sale", refSaleId: 100, note: null, userId: 1, createdAt: now - 3600 },
    ],
    api_create_sale: {
      sale: sales[0],
      items: [],
      receiptText,
    },
    api_last_sale: { sale: sales[0], items: [] },
    api_get_sale: { sale: sales[0], items: [] },
    api_refund_sale: { sale: { ...sales[0], status: "refunded" }, items: [], receiptText },
    api_sales_between: sales,
    api_reports: { from: now - 86400, to: now, z: zReport, top: [
      { name: "Coffee 250g", qty: 41, total: 2194500 },
      { name: "Olive Oil 1L", qty: 22, total: 2043600 },
      { name: "Milk 1L", qty: 88, total: 1056000 },
      { name: "Sugar 1kg", qty: 30, total: 321300 },
    ], daily: [
      { day: "2026-10-17", count: 37, gross: 486500, discount: 12300, taxTotal: 74120, total: 548320, cash: 312000, card: 236320 },
      { day: "2026-10-16", count: 44, gross: 611200, discount: 9000, taxTotal: 98200, total: 700400, cash: 402000, card: 298400 },
    ] },
    api_settings: settings,
    api_save_store_info: null,
    api_save_printer: null,
    api_set_language: null,
    api_backup: "/home/user/.local/share/pos/backup-1.db",
    api_print_receipt: null,
    api_print_test: null,
    api_receipt_text_for: receiptText,
  };
  await new Promise((r) => setTimeout(r, 40));
  if (!(cmd in fixtures)) throw new Error(`mock: unknown command ${cmd}`);
  return fixtures[cmd] as T;
}
