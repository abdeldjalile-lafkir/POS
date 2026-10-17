import * as React from "react";
import { Pencil, Plus } from "lucide-react";

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
import { formatAmount, formatDZD, parseToCentimes, rateLabel } from "@/lib/format";
import type { Category, Product, ProductInput } from "@/lib/types";

const RATES = [1900, 900, 0];

interface FormState {
  id: number | null;
  sku: string;
  barcode: string;
  name: string;
  nameAr: string;
  price: string;
  taxRateBp: number;
  unit: string;
  categoryId: number | null;
  isActive: boolean;
}

function emptyForm(): FormState {
  return {
    id: null,
    sku: "",
    barcode: "",
    name: "",
    nameAr: "",
    price: "0,00",
    taxRateBp: 0,
    unit: "pcs",
    categoryId: null,
    isActive: true,
  };
}

export function ProductsScreen() {
  const { t, rtl, toast } = useApp();
  const [items, setItems] = React.useState<Product[]>([]);
  const [categories, setCategories] = React.useState<Category[]>([]);
  const [query, setQuery] = React.useState("");
  const [showInactive, setShowInactive] = React.useState(false);
  const [form, setForm] = React.useState<FormState | null>(null);

  const load = React.useCallback(async () => {
    try {
      setItems(await api.listProducts(showInactive));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  }, [showInactive, toast]);

  React.useEffect(() => {
    void load();
    api.listCategories().then(setCategories).catch(() => {});
  }, [load]);

  const filtered = React.useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.nameAr.includes(q) ||
        p.sku.toLowerCase().includes(q) ||
        (p.barcode ?? "").includes(q),
    );
  }, [items, query]);

  const save = async () => {
    if (!form) return;
    if (!form.name.trim()) return toast(t("required"), "destructive");
    const input: ProductInput = {
      id: form.id,
      sku: form.sku,
      barcode: form.barcode.trim() ? form.barcode.trim() : null,
      name: form.name.trim(),
      nameAr: form.nameAr,
      categoryId: form.categoryId,
      price: parseToCentimes(form.price),
      taxRateBp: form.taxRateBp,
      unit: form.unit,
      isActive: form.isActive,
    };
    try {
      await api.saveProduct(input);
      setForm(null);
      await load();
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const toggleActive = async (p: Product) => {
    try {
      await api.setProductActive(p.id, !p.isActive);
      await load();
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{t("products")}</h2>
        <Badge variant="secondary">{filtered.length}</Badge>
        <div className="ms-auto flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={showInactive}
              onChange={(e) => setShowInactive(e.target.checked)}
            />
            {t("showInactive")}
          </label>
          <Input
            className="w-56"
            placeholder={t("search")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Button onClick={() => setForm(emptyForm())}>
            <Plus className="size-4" />
            {t("newProduct")}
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("barcode")}</TableHead>
              <TableHead>{t("price")}</TableHead>
              <TableHead>{t("taxRate")}</TableHead>
              <TableHead className="w-40" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((p) => (
              <TableRow key={p.id} className={p.isActive ? "" : "opacity-60"}>
                <TableCell className="font-medium">
                  {rtl && p.nameAr ? p.nameAr : p.name}
                  {!p.isActive && (
                    <Badge variant="outline" className="ms-2">
                      {t("inactive")}
                    </Badge>
                  )}
                </TableCell>
                <TableCell className="text-muted-foreground">{p.barcode}</TableCell>
                <TableCell>{formatDZD(p.price)}</TableCell>
                <TableCell>{rateLabel(p.taxRate)}</TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        setForm({
                          id: p.id,
                          sku: p.sku,
                          barcode: p.barcode ?? "",
                          name: p.name,
                          nameAr: p.nameAr,
                          price: formatAmount(p.price),
                          taxRateBp: p.taxRate,
                          unit: p.unit,
                          categoryId: p.categoryId,
                          isActive: p.isActive,
                        })
                      }
                    >
                      <Pencil className="size-3.5" />
                      {t("edit")}
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => toggleActive(p)}>
                      {p.isActive ? t("deactivate") : t("activate")}
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
            {filtered.length === 0 && (
              <TableRow>
                <TableCell colSpan={5} className="h-24 text-center text-muted-foreground">
                  {t("noData")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <Dialog open={form !== null} onOpenChange={(o) => !o && setForm(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{form?.id ? t("editProduct") : t("newProduct")}</DialogTitle>
          </DialogHeader>
          {form && (
            <div className="grid grid-cols-2 gap-4">
              <Field label={t("name")} className="col-span-2">
                <Input
                  autoFocus
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                />
              </Field>
              <Field label={t("nameAr")} className="col-span-2">
                <Input
                  dir="rtl"
                  value={form.nameAr}
                  onChange={(e) => setForm({ ...form, nameAr: e.target.value })}
                />
              </Field>
              <Field label={t("sku")}>
                <Input
                  value={form.sku}
                  onChange={(e) => setForm({ ...form, sku: e.target.value })}
                />
              </Field>
              <Field label={t("barcode")}>
                <Input
                  value={form.barcode}
                  onChange={(e) => setForm({ ...form, barcode: e.target.value })}
                />
              </Field>
              <Field label={t("price")}>
                <Input
                  value={form.price}
                  onChange={(e) => setForm({ ...form, price: e.target.value })}
                />
              </Field>
              <Field label={t("taxRate")}>
                <Select
                  value={String(form.taxRateBp)}
                  onValueChange={(v) => setForm({ ...form, taxRateBp: Number(v) })}
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
              </Field>
              <Field label={t("category")}>
                <Select
                  value={form.categoryId === null ? "none" : String(form.categoryId)}
                  onValueChange={(v) =>
                    setForm({ ...form, categoryId: v === "none" ? null : Number(v) })
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
              </Field>
              <Field label={t("unit")}>
                <Input
                  value={form.unit}
                  onChange={(e) => setForm({ ...form, unit: e.target.value })}
                />
              </Field>
              <label className="col-span-2 flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={form.isActive}
                  onChange={(e) => setForm({ ...form, isActive: e.target.checked })}
                />
                {t("active")}
              </label>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setForm(null)}>
              {t("cancel")}
            </Button>
            <Button onClick={save}>{t("save")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Field({
  label,
  className,
  children,
}: {
  label: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={`flex flex-col gap-2 ${className ?? ""}`}>
      <Label>{label}</Label>
      {children}
    </div>
  );
}
