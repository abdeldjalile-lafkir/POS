// Client-side mirror of the Rust cart/total engine, so the register can show
// the TVA breakdown live. All money is integer centimes.

export interface UiCartLine {
  key: string;
  productId: number | null;
  name: string;
  unitPrice: number; // centimes
  taxRateBp: number;
  qty: number;
  discountPct: number; // 0..90
}

export type SaleDiscount =
  | { kind: "none" }
  | { kind: "percent"; value: number } // percent
  | { kind: "amount"; value: number }; // centimes

export interface TaxLineTotal {
  rate: number;
  base: number;
  tax: number;
}

export interface Totals {
  gross: number;
  lineDiscounts: number;
  subtotal: number;
  saleDiscount: number;
  discountTotal: number;
  taxLines: TaxLineTotal[];
  taxTotal: number;
  total: number;
}

/** Round half away from zero (matches the Rust implementation). */
export function roundHalfUp(n: number, d: number): number {
  if (d === 0) return 0;
  return n >= 0 ? Math.floor((n + d / 2) / d) : -Math.floor((-n + d / 2) / d);
}

function percentOf(amount: number, bp: number): number {
  return roundHalfUp(amount * bp, 10000);
}

function lineDiscount(line: UiCartLine): number {
  if (line.discountPct <= 0) return 0;
  const gross = line.unitPrice * line.qty;
  const d = percentOf(gross, Math.round(line.discountPct * 100));
  return Math.min(Math.max(d, 0), gross);
}

function saleDiscountAmount(sd: SaleDiscount, subtotal: number): number {
  if (sd.kind === "percent") {
    const d = percentOf(subtotal, Math.round(sd.value * 100));
    return Math.min(Math.max(d, 0), subtotal);
  }
  if (sd.kind === "amount") {
    return Math.min(Math.max(Math.round(sd.value), 0), subtotal);
  }
  return 0;
}

/** Split `total` across `weights` proportionally, largest remainder. */
function allocate(total: number, weights: number[]): number[] {
  const n = weights.length;
  const out = new Array<number>(n).fill(0);
  if (n === 0 || total <= 0) return out;
  const sum = weights.reduce((a, b) => a + b, 0);
  if (sum <= 0) return out;

  let assigned = 0;
  const rems: Array<[number, number]> = [];
  for (let i = 0; i < n; i++) {
    const num = total * weights[i];
    const base = Math.floor(num / sum);
    out[i] = base;
    assigned += base;
    rems.push([i, num - base * sum]);
  }
  let left = total - assigned;
  rems.sort((a, b) => b[1] - a[1]);
  let k = 0;
  while (left > 0 && rems.length > 0) {
    out[rems[k % rems.length][0]] += 1;
    left -= 1;
    k += 1;
  }
  return out;
}

export function computeTotals(
  lines: UiCartLine[],
  sd: SaleDiscount,
): Totals {
  let gross = 0;
  let lineDiscounts = 0;
  const nets: number[] = [];
  for (const line of lines) {
    const g = line.unitPrice * line.qty;
    const d = lineDiscount(line);
    gross += g;
    lineDiscounts += d;
    nets.push(g - d);
  }
  const subtotal = nets.reduce((a, b) => a + b, 0);
  const discount = saleDiscountAmount(sd, subtotal);
  const alloc = allocate(discount, nets);

  const byRate = new Map<number, TaxLineTotal>();
  let taxTotal = 0;
  for (let i = 0; i < lines.length; i++) {
    const base = nets[i] - alloc[i];
    const rate = lines[i].taxRateBp;
    const tax = percentOf(base, rate);
    taxTotal += tax;
    const entry = byRate.get(rate) ?? { rate, base: 0, tax: 0 };
    entry.base += base;
    entry.tax += tax;
    byRate.set(rate, entry);
  }
  const taxLines = [...byRate.values()].sort((a, b) => b.rate - a.rate);

  return {
    gross,
    lineDiscounts,
    subtotal,
    saleDiscount: discount,
    discountTotal: lineDiscounts + discount,
    taxLines,
    taxTotal,
    total: subtotal - discount + taxTotal,
  };
}
