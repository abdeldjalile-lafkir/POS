import * as React from "react";

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
import { useApp } from "@/lib/app-context";
import { api, errorMessage } from "@/lib/api";
import { formatAmount, parseToCentimes, rateLabel } from "@/lib/format";
import type { Category, Product, ProductInput } from "@/lib/types";

const RATES = [0, 900, 1900];

interface Props {
  open: boolean;
  barcode: string;
  onOpenChange: (open: boolean) => void;
  onCreated: (product: Product) => void;
}

/**
 * Shown when a scanned barcode is not in the catalogue. Creates a product
 * (default TVA 0%) and hands it back so it can be added to the cart.
 */
export function QuickProductDialog({
  open,
  barcode,
  onOpenChange,
  onCreated,
}: Props) {
  const { t, rtl, toast } = useApp();
  const [name, setName] = React.useState("");
  const [price, setPrice] = React.useState("0,00");
  const [taxRateBp, setTaxRateBp] = React.useState(0);
  const [advanced, setAdvanced] = React.useState(false);
  const [nameAr, setNameAr] = React.useState("");
  const [unit, setUnit] = React.useState("pcs");
  const [categoryId, setCategoryId] = React.useState<number | null>(null);
  const [categories, setCategories] = React.useState<Category[]>([]);
  const [busy, setBusy] = React.useState(false);

  // Reset the form whenever a new barcode arrives.
  React.useEffect(() => {
    if (!open) return;
    setName("");
    setPrice("0,00");
    setTaxRateBp(0);
    setAdvanced(false);
    setNameAr("");
    setUnit("pcs");
    setCategoryId(null);
    api.listCategories().then(setCategories).catch(() => {});
  }, [open, barcode]);

  const save = async () => {
    if (!name.trim()) return toast(t("required"), "destructive");
    const input: ProductInput = {
      id: null,
      sku: "",
      barcode: barcode.trim() ? barcode.trim() : null,
      name: name.trim(),
      nameAr,
      categoryId,
      price: parseToCentimes(price),
      taxRateBp,
      unit: unit.trim() || "pcs",
      isActive: true,
    };
    setBusy(true);
    try {
      const id = await api.saveProduct(input);
      const created = await api.getProduct(id);
      if (created) {
        onCreated(created);
        toast(`${t("saved")} · ${created.name}`);
        onOpenChange(false);
      } else {
        toast(t("saved"));
        onOpenChange(false);
      }
    } catch (e) {
      toast(errorMessage(e), "destructive");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("quickAddTitle")}</DialogTitle>
          <p className="text-sm text-muted-foreground">{t("quickAddHint")}</p>
        </DialogHeader>

        <div className="grid grid-cols-2 gap-4">
          <div className="col-span-2 flex flex-col gap-2">
            <Label>{t("barcode")}</Label>
            <Input value={barcode} readOnly className="bg-muted/50 font-mono" />
          </div>

          <div className="col-span-2 flex flex-col gap-2">
            <Label>{t("name")}</Label>
            <Input
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void save();
              }}
            />
          </div>

          <div className="flex flex-col gap-2">
            <Label>{t("price")}</Label>
            <Input value={price} onChange={(e) => setPrice(e.target.value)} />
          </div>

          <div className="flex flex-col gap-2">
            <Label>{t("taxRate")}</Label>
            <Select
              value={String(taxRateBp)}
              onValueChange={(v) => setTaxRateBp(Number(v))}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {RATES.map((r) => (
                  <SelectItem key={r} value={String(r)}>
                    {rateLabel(r)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <button
          type="button"
          className="w-fit text-sm text-muted-foreground underline-offset-4 hover:underline"
          onClick={() => setAdvanced((a) => !a)}
        >
          {t("advanced")}
        </button>

        {advanced && (
          <div className="grid grid-cols-2 gap-4">
            <div className="col-span-2 flex flex-col gap-2">
              <Label>{t("nameAr")}</Label>
              <Input
                dir="rtl"
                value={nameAr}
                onChange={(e) => setNameAr(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-2">
              <Label>{t("unit")}</Label>
              <Input value={unit} onChange={(e) => setUnit(e.target.value)} />
            </div>
            <div className="flex flex-col gap-2">
              <Label>{t("category")}</Label>
              <Select
                value={categoryId === null ? "none" : String(categoryId)}
                onValueChange={(v) =>
                  setCategoryId(v === "none" ? null : Number(v))
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">—</SelectItem>
                  {categories.map((c) => (
                    <SelectItem key={c.id} value={String(c.id)}>
                      {rtl && c.nameAr ? c.nameAr : c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        )}

        <DialogFooter>
          <span className="me-auto text-sm text-muted-foreground">
            {t("price")}: {formatAmount(parseToCentimes(price))} DA
          </span>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("cancel")}
          </Button>
          <Button onClick={save} disabled={busy}>
            {t("saveAndAdd")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
