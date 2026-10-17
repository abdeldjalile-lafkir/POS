import * as React from "react";
import { Minus, Plus, Printer, ScanLine, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";
import { formatAmount, formatDZD, parseToCentimes, rateLabel } from "@/lib/format";
import type { CartInput, PaymentMethod, Product } from "@/lib/types";
import {
  computeTotals,
  type SaleDiscount,
  type UiCartLine,
} from "@/lib/totals";

let lineSeq = 0;

export function Register() {
  const { t, rtl, toast } = useApp();
  const [query, setQuery] = React.useState("");
  const [results, setResults] = React.useState<Product[]>([]);
  const [lines, setLines] = React.useState<UiCartLine[]>([]);
  const [saleDiscount, setSaleDiscount] = React.useState<SaleDiscount>({
    kind: "none",
  });
  const [scan, setScan] = React.useState("");
  const searchRef = React.useRef<HTMLInputElement>(null);

  const [payOpen, setPayOpen] = React.useState(false);
  const [payMethod, setPayMethod] = React.useState<PaymentMethod>("cash");
  const [tendered, setTendered] = React.useState("");
  const [receipt, setReceipt] = React.useState<{ id: number; text: string } | null>(
    null,
  );

  const totals = React.useMemo(
    () => computeTotals(lines, saleDiscount),
    [lines, saleDiscount],
  );

  const runSearch = React.useCallback(
    async (q: string) => {
      const term = q.trim();
      if (!term) {
        setResults([]);
        return;
      }
      try {
        setResults(await api.searchProducts(term, 12));
      } catch (e) {
        toast(errorMessage(e), "destructive");
      }
    },
    [toast],
  );

  const addProduct = React.useCallback((p: Product) => {
    setLines((prev) => {
      const idx = prev.findIndex((l) => l.productId === p.id);
      if (idx >= 0) {
        const copy = [...prev];
        copy[idx] = { ...copy[idx], qty: copy[idx].qty + 1 };
        return copy;
      }
      return [
        ...prev,
        {
          key: `l${++lineSeq}`,
          productId: p.id,
          name: p.name,
          unitPrice: p.price,
          taxRateBp: p.taxRate,
          qty: 1,
          discountPct: 0,
        },
      ];
    });
    setQuery("");
    setResults([]);
    searchRef.current?.focus();
  }, []);

  const handleCode = React.useCallback(
    async (code: string) => {
      try {
        const p = await api.findByBarcode(code);
        if (p) {
          addProduct(p);
        } else {
          toast(`${t("notInCatalogue")}: ${code}`, "destructive");
        }
      } catch (e) {
        toast(errorMessage(e), "destructive");
      }
    },
    [addProduct, t, toast],
  );

  // Global scanner capture when no field is focused.
  React.useEffect(() => {
    let buf = "";
    let last = 0;
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement;
      const typing =
        el instanceof HTMLInputElement ||
        el instanceof HTMLTextAreaElement ||
        el instanceof HTMLSelectElement;
      if (typing) return;
      const now = performance.now();
      if (now - last > 60) buf = "";
      last = now;
      if (e.key === "Enter") {
        if (buf.length >= 4) void handleCode(buf);
        buf = "";
        return;
      }
      if (e.key.length === 1) buf += e.key;
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleCode]);

  const onSearchKey = (e: React.KeyboardEvent) => {
    if (e.key !== "Enter") return;
    const term = query.trim();
    if (!term) return;
    e.preventDefault();
    if (term.length >= 6 && /^[0-9A-Za-z-]+$/.test(term)) {
      void handleCode(term);
    } else if (results.length > 0) {
      addProduct(results[0]);
    }
  };

  const toCartInput = (): CartInput => ({
    lines: lines.map((l) => ({
      productId: l.productId,
      name: l.name,
      unitPrice: l.unitPrice,
      taxRateBp: l.taxRateBp,
      qty: l.qty,
      discount:
        l.discountPct > 0
          ? { kind: "percent" as const, value: l.discountPct }
          : null,
    })),
    saleDiscount:
      saleDiscount.kind === "none"
        ? null
        : { kind: saleDiscount.kind, value: saleDiscount.value },
  });

  const confirmPayment = async () => {
    const tenderedCentimes =
      payMethod === "cash" ? parseToCentimes(tendered) : totals.total;
    if (payMethod === "cash" && tenderedCentimes < totals.total) {
      return toast(t("insufficient"), "destructive");
    }
    try {
      const res = await api.createSale(toCartInput(), {
        method: payMethod,
        tendered: tenderedCentimes,
      });
      setPayOpen(false);
      setLines([]);
      setSaleDiscount({ kind: "none" });
      setTendered("");
      setReceipt({ id: res.sale.id, text: res.receiptText });
      toast(`${t("saved")} · ${res.sale.receiptNo}`);
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  return (
    <div className="flex h-full min-h-0 gap-4 p-4">
      {/* Scan / search panel */}
      <div className="flex w-96 shrink-0 flex-col gap-3">
        <div className="flex items-center gap-2">
          <ScanLine className="size-5 text-primary" />
          <Input
            ref={searchRef}
            autoFocus
            value={query}
            placeholder={t("scanOrSearch")}
            onChange={(e) => {
              setQuery(e.target.value);
              void runSearch(e.target.value);
            }}
            onKeyDown={onSearchKey}
          />
        </div>

        <div className="flex items-center gap-2">
          <Input
            value={scan}
            placeholder={t("simulateScan")}
            onChange={(e) => setScan(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && scan.trim()) {
                void handleCode(scan.trim());
                setScan("");
              }
            }}
          />
        </div>

        <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
          {results.length === 0 ? (
            <div className="p-4 text-sm text-muted-foreground">{t("noResults")}</div>
          ) : (
            <ul className="divide-y">
              {results.map((p) => (
                <li key={p.id}>
                  <button
                    onClick={() => addProduct(p)}
                    className="flex w-full items-center justify-between px-3 py-2 text-start hover:bg-accent"
                  >
                    <span className="truncate">{rtl && p.nameAr ? p.nameAr : p.name}</span>
                    <span className="text-sm text-muted-foreground">
                      {formatDZD(p.price)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {/* Cart */}
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">{t("cart")}</h2>
          <Button
            variant="ghost"
            size="sm"
            disabled={lines.length === 0}
            onClick={() => {
              setLines([]);
              setSaleDiscount({ kind: "none" });
            }}
          >
            {t("newSale")}
          </Button>
        </div>

        <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
          {lines.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {t("emptyCart")}
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("item")}</TableHead>
                  <TableHead className="w-32 text-center">{t("qty")}</TableHead>
                  <TableHead className="w-24">{t("price")}</TableHead>
                  <TableHead className="w-24">{t("lineDiscountPct")}</TableHead>
                  <TableHead className="w-28 text-end">{t("lineTotal")}</TableHead>
                  <TableHead className="w-10" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {lines.map((l, i) => (
                  <TableRow key={l.key}>
                    <TableCell className="font-medium">{l.name}</TableCell>
                    <TableCell>
                      <div className="flex items-center justify-center gap-1">
                        <Button
                          variant="outline"
                          size="icon"
                          className="size-7"
                          onClick={() =>
                            setLines((prev) =>
                              prev.map((x, j) =>
                                j === i ? { ...x, qty: Math.max(1, x.qty - 1) } : x,
                              ),
                            )
                          }
                        >
                          <Minus className="size-3" />
                        </Button>
                        <span className="w-8 text-center tabular-nums">{l.qty}</span>
                        <Button
                          variant="outline"
                          size="icon"
                          className="size-7"
                          onClick={() =>
                            setLines((prev) =>
                              prev.map((x, j) =>
                                j === i ? { ...x, qty: x.qty + 1 } : x,
                              ),
                            )
                          }
                        >
                          <Plus className="size-3" />
                        </Button>
                      </div>
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {formatDZD(l.unitPrice)}
                    </TableCell>
                    <TableCell>
                      <Input
                        type="number"
                        min={0}
                        max={90}
                        value={l.discountPct || ""}
                        className="h-7 w-16"
                        onChange={(e) => {
                          const v = Math.min(90, Math.max(0, Number(e.target.value) || 0));
                          setLines((prev) =>
                            prev.map((x, j) => (j === i ? { ...x, discountPct: v } : x)),
                          );
                        }}
                      />
                    </TableCell>
                    <TableCell className="text-end tabular-nums">
                      {formatDZD(
                        l.unitPrice * l.qty -
                          (l.discountPct > 0
                            ? Math.round(
                                (l.unitPrice * l.qty * (l.discountPct * 100)) / 10000,
                              )
                            : 0),
                      )}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-7 text-destructive"
                        onClick={() =>
                          setLines((prev) => prev.filter((_, j) => j !== i))
                        }
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>

        {/* Totals */}
        <div className="rounded-lg border bg-card p-4">
          <div className="mb-2 flex items-center gap-2 text-sm">
            <span className="text-muted-foreground">{t("saleDiscount")}</span>
            <div className="flex gap-1">
              {(["none", "percent", "amount"] as const).map((k) => (
                <Button
                  key={k}
                  size="sm"
                  variant={saleDiscount.kind === k ? "default" : "outline"}
                  onClick={() =>
                    setSaleDiscount(
                      k === "none"
                        ? { kind: "none" }
                        : k === "percent"
                          ? { kind: "percent", value: 5 }
                          : { kind: "amount", value: 0 },
                    )
                  }
                >
                  {k === "none" ? "—" : k === "percent" ? t("percent") : t("amount")}
                </Button>
              ))}
            </div>
            {saleDiscount.kind === "percent" && (
              <Input
                type="number"
                className="h-7 w-20"
                value={saleDiscount.value}
                onChange={(e) =>
                  setSaleDiscount({ kind: "percent", value: Number(e.target.value) || 0 })
                }
              />
            )}
            {saleDiscount.kind === "amount" && (
              <Input
                className="h-7 w-24"
                value={formatAmount(saleDiscount.value)}
                onChange={(e) =>
                  setSaleDiscount({ kind: "amount", value: parseToCentimes(e.target.value) })
                }
              />
            )}
          </div>

          <Separator className="my-2" />
          <div className="space-y-1 text-sm">
            <Row label={t("subtotal")} value={formatDZD(totals.subtotal)} />
            {totals.discountTotal > 0 && (
              <Row label={t("discount")} value={`-${formatDZD(totals.discountTotal)}`} />
            )}
            {totals.taxLines.map((tl) => (
              <Row
                key={tl.rate}
                label={`${t("vat")} ${rateLabel(tl.rate)}`}
                value={formatDZD(tl.tax)}
              />
            ))}
            <Separator className="my-2" />
            <div className="flex items-center justify-between text-lg font-semibold">
              <span>{t("total")}</span>
              <span className="tabular-nums">{formatDZD(totals.total)}</span>
            </div>
          </div>

          <Button
            size="lg"
            className="mt-3 w-full"
            disabled={lines.length === 0}
            onClick={() => {
              setPayMethod("cash");
              setTendered(formatAmount(totals.total));
              setPayOpen(true);
            }}
          >
            {t("pay")}
          </Button>
        </div>
      </div>

      {/* Payment dialog */}
      <Dialog open={payOpen} onOpenChange={setPayOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("payment")}</DialogTitle>
          </DialogHeader>
          <div className="flex gap-2">
            {(["cash", "card", "other"] as PaymentMethod[]).map((m) => (
              <Button
                key={m}
                variant={payMethod === m ? "default" : "outline"}
                onClick={() => setPayMethod(m)}
                className="flex-1"
              >
                {t(m)}
              </Button>
            ))}
          </div>
          <div className="flex items-center justify-between text-lg">
            <span>{t("total")}</span>
            <span className="font-semibold tabular-nums">{formatDZD(totals.total)}</span>
          </div>
          {payMethod === "cash" && (
            <>
              <div className="flex flex-col gap-2">
                <Label>{t("tendered")}</Label>
                <Input
                  autoFocus
                  value={tendered}
                  onChange={(e) => setTendered(e.target.value)}
                />
              </div>
              <div className="flex items-center justify-between">
                <span>{t("change")}</span>
                <span className="font-semibold tabular-nums">
                  {formatDZD(Math.max(0, parseToCentimes(tendered) - totals.total))}
                </span>
              </div>
              {parseToCentimes(tendered) < totals.total && (
                <p className="text-sm text-destructive">{t("insufficient")}</p>
              )}
            </>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setPayOpen(false)}>
              {t("cancel")}
            </Button>
            <Button onClick={confirmPayment}>{t("confirm")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Receipt dialog */}
      <Dialog open={receipt !== null} onOpenChange={(o) => !o && setReceipt(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("receipt")}</DialogTitle>
          </DialogHeader>
          <pre className="max-h-96 overflow-auto rounded-md border bg-muted/40 p-3 text-xs leading-relaxed">
            {receipt?.text}
          </pre>
          <DialogFooter>
            <Badge variant="secondary" className="me-auto">
              {receipt ? `#${receipt.id}` : ""}
            </Badge>
            <Button
              variant="outline"
              onClick={async () => {
                if (!receipt) return;
                try {
                  await api.printReceipt(receipt.id);
                  toast(t("saved"));
                } catch (e) {
                  toast(errorMessage(e), "destructive");
                }
              }}
            >
              <Printer className="size-4" />
              {t("print")}
            </Button>
            <Button onClick={() => setReceipt(null)}>{t("close")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-muted-foreground">{label}</span>
      <span className="tabular-nums">{value}</span>
    </div>
  );
}
