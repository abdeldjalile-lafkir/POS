// Formatting helpers. Money is centimes; display uses Algerian conventions:
// narrow space for thousands, a comma for decimals, and the "DA" suffix.

const NARROW = "\u202f";

function group(dinars: number): string {
  return dinars.toLocaleString("fr-FR").replace(/[\u00a0\u202f ]/g, NARROW);
}

/** `123456` -> `1 234,56 DA` */
export function formatDZD(centimes: number): string {
  return `${formatAmount(centimes)} DA`;
}

/** `123456` -> `1 234,56` */
export function formatAmount(centimes: number): string {
  const neg = centimes < 0;
  const a = Math.abs(Math.round(centimes));
  const dinars = Math.floor(a / 100);
  const cents = a % 100;
  return `${neg ? "-" : ""}${group(dinars)},${String(cents).padStart(2, "0")}`;
}

/** Parse user text (`1 234,56`, `1234.5`, `1200`) into centimes. */
export function parseToCentimes(input: string): number {
  const cleaned = input.replace(/[\s\u00a0\u202f]/g, "").replace(",", ".");
  if (!/^-?\d*(\.\d*)?$/.test(cleaned) || cleaned === "" || cleaned === "-") return 0;
  return Math.round(parseFloat(cleaned) * 100);
}

/** Basis points to a label: 1900 -> `19%`, 750 -> `7,50%`. */
export function rateLabel(bp: number): string {
  if (bp % 100 === 0) return `${bp / 100}%`;
  return `${Math.floor(bp / 100)},${String(bp % 100).padStart(2, "0")}%`;
}

export function formatDateTime(unix: number): string {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleString();
}

export function formatDate(unix: number): string {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleDateString();
}

export function nowPlusOne(): number {
  return Math.floor(Date.now() / 1000) + 1;
}

export function dayStartOffset(days: number): number {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() - days);
  return Math.floor(d.getTime() / 1000);
}

export function monthStart(): number {
  const d = new Date();
  return Math.floor(new Date(d.getFullYear(), d.getMonth(), 1).getTime() / 1000);
}
