import * as React from "react";
import { Database, Download, Printer, Save } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import { LANGS, langName, type Lang } from "@/lib/i18n";
import type { AppSettings, PrinterSettings, StoreInfo } from "@/lib/types";

export function SettingsScreen() {
  const { t, lang, setLang, toast } = useApp();
  const [store, setStore] = React.useState<StoreInfo | null>(null);
  const [printer, setPrinter] = React.useState<PrinterSettings | null>(null);
  const [csv, setCsv] = React.useState("");
  const [importing, setImporting] = React.useState(false);

  React.useEffect(() => {
    api
      .settings()
      .then((s: AppSettings) => {
        setStore(s.store);
        setPrinter(s.printer);
      })
      .catch((e) => toast(errorMessage(e), "destructive"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const saveStore = async () => {
    if (!store) return;
    try {
      await api.saveStoreInfo(store);
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const savePrinter = async () => {
    if (!printer) return;
    try {
      await api.savePrinter(printer);
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const changeLang = async (l: Lang) => {
    setLang(l);
    try {
      await api.setLanguage(l);
    } catch {
      /* non-fatal */
    }
  };

  const doImport = async (source: "manual" | "bundled") => {
    setImporting(true);
    try {
      const data = source === "bundled" ? await api.bundledCatalogue() : csv;
      const s = await api.importCatalogue(data);
      toast(
        `${t("importDone")} · +${s.productsAdded} / ~${s.productsUpdated} · ${s.categoriesAdded} ${t("category")}`,
      );
    } catch (e) {
      toast(errorMessage(e), "destructive");
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="flex h-full flex-col gap-4 overflow-auto p-4">
      <h2 className="text-lg font-semibold">{t("settings")}</h2>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("storeInfo")}</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-2 gap-4">
            {store && (
              <>
                <F label={t("storeName")} className="col-span-2">
                  <Input
                    value={store.name}
                    onChange={(e) => setStore({ ...store, name: e.target.value })}
                  />
                </F>
                <F label={t("address")} className="col-span-2">
                  <Input
                    value={store.address}
                    onChange={(e) => setStore({ ...store, address: e.target.value })}
                  />
                </F>
                <F label={t("nif")}>
                  <Input
                    value={store.nif}
                    onChange={(e) => setStore({ ...store, nif: e.target.value })}
                  />
                </F>
                <F label={t("nis")}>
                  <Input
                    value={store.nis}
                    onChange={(e) => setStore({ ...store, nis: e.target.value })}
                  />
                </F>
                <F label={t("rc")}>
                  <Input
                    value={store.rc}
                    onChange={(e) => setStore({ ...store, rc: e.target.value })}
                  />
                </F>
                <F label={t("ai")}>
                  <Input
                    value={store.ai}
                    onChange={(e) => setStore({ ...store, ai: e.target.value })}
                  />
                </F>
                <F label={t("phone")}>
                  <Input
                    value={store.phone}
                    onChange={(e) => setStore({ ...store, phone: e.target.value })}
                  />
                </F>
                <F label={t("footer")}>
                  <Input
                    value={store.receiptFooter}
                    onChange={(e) =>
                      setStore({ ...store, receiptFooter: e.target.value })
                    }
                  />
                </F>
                <div className="col-span-2">
                  <Button onClick={saveStore}>
                    <Save className="size-4" />
                    {t("save")}
                  </Button>
                </div>
              </>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("printer")}</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-2 gap-4">
            {printer && (
              <>
                <F label={t("transport")} className="col-span-2">
                  <Select
                    value={printer.transport}
                    onValueChange={(v) => setPrinter({ ...printer, transport: v })}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="preview">{t("previewOnly")}</SelectItem>
                      <SelectItem value="file">{t("deviceFile")}</SelectItem>
                      <SelectItem value="network">{t("network")}</SelectItem>
                    </SelectContent>
                  </Select>
                </F>
                {printer.transport === "network" && (
                  <>
                    <F label={t("host")}>
                      <Input
                        value={printer.host}
                        onChange={(e) =>
                          setPrinter({ ...printer, host: e.target.value })
                        }
                      />
                    </F>
                    <F label={t("port")}>
                      <Input
                        value={printer.port}
                        onChange={(e) =>
                          setPrinter({ ...printer, port: e.target.value })
                        }
                      />
                    </F>
                  </>
                )}
                {printer.transport === "file" && (
                  <F label={t("devicePath")} className="col-span-2">
                    <Input
                      value={printer.device}
                      onChange={(e) =>
                        setPrinter({ ...printer, device: e.target.value })
                      }
                    />
                  </F>
                )}
                <F label={t("paperWidth")}>
                  <Select
                    value={String(printer.paperWidthMm)}
                    onValueChange={(v) =>
                      setPrinter({ ...printer, paperWidthMm: Number(v) })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="58">58 mm</SelectItem>
                      <SelectItem value="80">80 mm</SelectItem>
                    </SelectContent>
                  </Select>
                </F>
                <F label={t("scanGap")}>
                  <Input
                    type="number"
                    value={printer.scanMaxGapMs}
                    onChange={(e) =>
                      setPrinter({
                        ...printer,
                        scanMaxGapMs: Number(e.target.value) || 0,
                      })
                    }
                  />
                </F>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={printer.autoCut}
                    onChange={(e) =>
                      setPrinter({ ...printer, autoCut: e.target.checked })
                    }
                  />
                  {t("autoCut")}
                </label>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={printer.drawerKick}
                    onChange={(e) =>
                      setPrinter({ ...printer, drawerKick: e.target.checked })
                    }
                  />
                  {t("cashDrawer")}
                </label>
                <div className="col-span-2 flex gap-2">
                  <Button onClick={savePrinter}>
                    <Save className="size-4" />
                    {t("save")}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={async () => {
                      try {
                        await api.printTest();
                        toast(t("saved"));
                      } catch (e) {
                        toast(errorMessage(e), "destructive");
                      }
                    }}
                  >
                    <Printer className="size-4" />
                    {t("testPrint")}
                  </Button>
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">{t("language")}</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center gap-3">
          {LANGS.map((l) => (
            <Button
              key={l}
              variant={lang === l ? "default" : "outline"}
              onClick={() => changeLang(l)}
            >
              {langName(l)}
            </Button>
          ))}
          <Button
            variant="outline"
            className="ms-auto"
            onClick={async () => {
              try {
                const path = await api.backup();
                toast(`${t("backup")}: ${path}`);
              } catch (e) {
                toast(errorMessage(e), "destructive");
              }
            }}
          >
            <Database className="size-4" />
            {t("backupDb")}
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">{t("catalogueImport")}</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <p className="text-sm text-muted-foreground">{t("catalogueImportHint")}</p>
          <textarea
            dir="ltr"
            spellCheck={false}
            value={csv}
            onChange={(e) => setCsv(e.target.value)}
            placeholder="barcode,name,name_ar,category,category_ar,price,vat,unit,stock"
            className="h-40 w-full resize-y rounded-md border border-input bg-transparent p-3 font-mono text-xs shadow-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => doImport("manual")} disabled={importing || !csv.trim()}>
              {t("importCsv")}
            </Button>
            <Button
              variant="outline"
              onClick={() => doImport("bundled")}
              disabled={importing}
            >
              {t("loadBundled")}
            </Button>
            <Button
              variant="secondary"
              className="ms-auto"
              onClick={async () => {
                try {
                  const path = await api.exportCatalogue();
                  toast(`${t("exportProducts")}: ${path}/catalogue.csv`);
                } catch (e) {
                  toast(errorMessage(e), "destructive");
                }
              }}
            >
              <Download className="size-4" />
              {t("exportProducts")}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function F({
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
