import * as React from "react";
import { PackageMinus } from "lucide-react";

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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
import { formatDateTime } from "@/lib/format";
import type { StockMovement, StockReason, StockRow } from "@/lib/types";

const REASONS: StockReason[] = ["purchase", "adjustment", "return"];

export function InventoryScreen() {
  const { t, rtl, toast } = useApp();
  const [rows, setRows] = React.useState<StockRow[]>([]);
  const [threshold, setThreshold] = React.useState(5);
  const [lowOnly, setLowOnly] = React.useState(false);
  const [selected, setSelected] = React.useState<StockRow | null>(null);
  const [movements, setMovements] = React.useState<StockMovement[]>([]);
  const [delta, setDelta] = React.useState("0");
  const [reason, setReason] = React.useState<StockReason>("purchase");
  const [note, setNote] = React.useState("");

  const load = React.useCallback(async () => {
    try {
      setRows(lowOnly ? await api.lowStock(threshold) : await api.stockOverview());
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  }, [lowOnly, threshold, toast]);

  React.useEffect(() => {
    void load();
  }, [load]);

  const openAdjust = async (row: StockRow) => {
    setSelected(row);
    setDelta("0");
    setReason("purchase");
    setNote("");
    try {
      setMovements(await api.movementsFor(row.product.id));
    } catch {
      setMovements([]);
    }
  };

  const apply = async () => {
    if (!selected) return;
    const d = Number.parseInt(delta.replace("+", ""), 10);
    if (!Number.isFinite(d) || d === 0) return;
    try {
      const level = await api.adjustStock({
        productId: selected.product.id,
        delta: d,
        reason,
        note: note.trim() ? note.trim() : null,
      });
      toast(`${t("onHand")}: ${level}`);
      setMovements(await api.movementsFor(selected.product.id));
      await load();
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{t("inventory")}</h2>
        <div className="ms-auto flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm">
            {t("threshold")}
            <Input
              type="number"
              className="h-8 w-20"
              value={threshold}
              onChange={(e) => setThreshold(Number(e.target.value) || 0)}
            />
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={lowOnly}
              onChange={(e) => setLowOnly(e.target.checked)}
            />
            {t("lowStockOnly")}
          </label>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("name")}</TableHead>
              <TableHead className="w-32">{t("onHand")}</TableHead>
              <TableHead className="w-32">{t("unit")}</TableHead>
              <TableHead className="w-32" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.product.id}>
                <TableCell className="font-medium">
                  {rtl && row.product.nameAr ? row.product.nameAr : row.product.name}
                </TableCell>
                <TableCell>
                  <Badge variant={row.quantity <= threshold ? "warning" : "secondary"}>
                    {row.quantity}
                  </Badge>
                </TableCell>
                <TableCell className="text-muted-foreground">{row.product.unit}</TableCell>
                <TableCell>
                  <Button variant="outline" size="sm" onClick={() => openAdjust(row)}>
                    <PackageMinus className="size-3.5" />
                    {t("adjust")}
                  </Button>
                </TableCell>
              </TableRow>
            ))}
            {rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={4} className="h-24 text-center text-muted-foreground">
                  {t("noData")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <Dialog open={selected !== null} onOpenChange={(o) => !o && setSelected(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {t("adjust")} · {selected?.product.name}
            </DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-4">
            <div className="flex flex-col gap-2">
              <Label>{t("delta")}</Label>
              <Input value={delta} onChange={(e) => setDelta(e.target.value)} />
            </div>
            <div className="flex flex-col gap-2">
              <Label>{t("reason")}</Label>
              <Select value={reason} onValueChange={(v) => setReason(v as StockReason)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {REASONS.map((r) => (
                    <SelectItem key={r} value={r}>
                      {t(r === "return" ? "ret" : r)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="col-span-2 flex flex-col gap-2">
              <Label>{t("note")}</Label>
              <Input value={note} onChange={(e) => setNote(e.target.value)} />
            </div>
          </div>

          <div className="mt-2">
            <div className="mb-2 text-sm font-medium">{t("movements")}</div>
            <div className="max-h-48 overflow-auto rounded-md border">
              {movements.length === 0 ? (
                <div className="p-3 text-sm text-muted-foreground">{t("noData")}</div>
              ) : (
                <ul className="divide-y text-sm">
                  {movements.map((m) => (
                    <li
                      key={m.id}
                      className="flex items-center justify-between px-3 py-1.5"
                    >
                      <span className="text-muted-foreground">
                        {formatDateTime(m.createdAt)}
                      </span>
                      <span className="text-muted-foreground">{t(m.reason)}</span>
                      <span className="tabular-nums">
                        {m.delta > 0 ? `+${m.delta}` : m.delta}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setSelected(null)}>
              {t("cancel")}
            </Button>
            <Button onClick={apply}>{t("apply")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
