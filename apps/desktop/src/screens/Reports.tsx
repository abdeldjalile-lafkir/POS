import * as React from "react";
import { Printer, Undo2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import {
  dayStartOffset,
  formatDateTime,
  formatDZD,
  monthStart,
  nowPlusOne,
  rateLabel,
} from "@/lib/format";
import type { ReportsBundle, Sale } from "@/lib/types";

type PeriodKind = "today" | "week" | "month";

function bounds(p: PeriodKind): [number, number] {
  const to = nowPlusOne();
  if (p === "today") return [dayStartOffset(0), to];
  if (p === "week") return [dayStartOffset(6), to];
  return [monthStart(), to];
}

export function ReportsScreen() {
  const { t, toast } = useApp();
  const [period, setPeriod] = React.useState<PeriodKind>("today");
  const [report, setReport] = React.useState<ReportsBundle | null>(null);
  const [sales, setSales] = React.useState<Sale[]>([]);

  const load = React.useCallback(async () => {
    const [from, to] = bounds(period);
    try {
      const [r, s] = await Promise.all([
        api.reports(from, to),
        api.salesBetween(from, to),
      ]);
      setReport(r);
      setSales(s.slice().reverse());
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  }, [period, toast]);

  React.useEffect(() => {
    void load();
  }, [load]);

  const refund = async (s: Sale) => {
    try {
      await api.refundSale(s.id);
      toast(`${t("refunds")} · ${s.receiptNo}`);
      await load();
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const print = async (s: Sale) => {
    try {
      await api.printReceipt(s.id);
      toast(t("saved"));
    } catch (e) {
      toast(errorMessage(e), "destructive");
    }
  };

  const z = report?.z;

  return (
    <div className="flex h-full flex-col gap-4 overflow-auto p-4">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{t("reports")}</h2>
        <div className="ms-auto flex gap-1">
          {(["today", "week", "month"] as PeriodKind[]).map((p) => (
            <Button
              key={p}
              size="sm"
              variant={period === p ? "default" : "outline"}
              onClick={() => setPeriod(p)}
            >
              {p === "today" ? t("today") : p === "week" ? t("thisWeek") : t("thisMonth")}
            </Button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
        <Stat title={t("salesCount")} value={String(z?.count ?? 0)} />
        <Stat title={t("netTotal")} value={formatDZD(z?.total ?? 0)} />
        <Stat title={t("vatCollected")} value={formatDZD(z?.taxTotal ?? 0)} />
        <Stat title={t("cashExpected")} value={formatDZD(z?.cashExpected ?? 0)} />
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("zReport")}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm">
            <KV label={t("gross")} value={formatDZD(z?.gross ?? 0)} />
            <KV label={t("discounts")} value={`-${formatDZD(z?.discount ?? 0)}`} />
            <KV label={t("vatCollected")} value={formatDZD(z?.taxTotal ?? 0)} />
            <KV label={t("netTotal")} value={formatDZD(z?.total ?? 0)} />
            <KV label={t("cash")} value={formatDZD(z?.cashTotal ?? 0)} />
            <KV label={t("card")} value={formatDZD(z?.cardTotal ?? 0)} />
            <KV label={t("refunds")} value={formatDZD(z?.refundsTotal ?? 0)} />
            <KV
              label={t("cashExpected")}
              value={formatDZD(z?.cashExpected ?? 0)}
              strong
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("byRate")}</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("taxRate")}</TableHead>
                  <TableHead>{t("base")}</TableHead>
                  <TableHead className="text-end">{t("vat")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(z?.byRate ?? []).map((r) => (
                  <TableRow key={r.rate}>
                    <TableCell>{rateLabel(r.rate)}</TableCell>
                    <TableCell>{formatDZD(r.base)}</TableCell>
                    <TableCell className="text-end">{formatDZD(r.tax)}</TableCell>
                  </TableRow>
                ))}
                {(z?.byRate ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={3} className="text-center text-muted-foreground">
                      {t("noData")}
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("topProducts")}</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("item")}</TableHead>
                  <TableHead>{t("qty")}</TableHead>
                  <TableHead className="text-end">{t("lineTotal")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(report?.top ?? []).map((p, i) => (
                  <TableRow key={i}>
                    <TableCell>{p.name}</TableCell>
                    <TableCell>{p.qty}</TableCell>
                    <TableCell className="text-end">{formatDZD(p.total)}</TableCell>
                  </TableRow>
                ))}
                {(report?.top ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={3} className="text-center text-muted-foreground">
                      {t("noData")}
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("dailyTotals")}</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("day")}</TableHead>
                  <TableHead>{t("salesCount")}</TableHead>
                  <TableHead className="text-end">{t("netTotal")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(report?.daily ?? []).map((d) => (
                  <TableRow key={d.day}>
                    <TableCell>{d.day}</TableCell>
                    <TableCell>{d.count}</TableCell>
                    <TableCell className="text-end">{formatDZD(d.total)}</TableCell>
                  </TableRow>
                ))}
                {(report?.daily ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={3} className="text-center text-muted-foreground">
                      {t("noData")}
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">{t("salesCount")}</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("receipt")}</TableHead>
                <TableHead>{t("day")}</TableHead>
                <TableHead>{t("cashier")}</TableHead>
                <TableHead>{t("total")}</TableHead>
                <TableHead className="w-48" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {sales.map((s) => (
                <TableRow key={s.id}>
                  <TableCell className="font-mono text-xs">{s.receiptNo}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatDateTime(s.createdAt)}
                  </TableCell>
                  <TableCell>{s.cashierName}</TableCell>
                  <TableCell>{formatDZD(s.total)}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Badge
                        variant={s.status === "completed" ? "success" : "destructive"}
                      >
                        {s.status}
                      </Badge>
                      <Button variant="ghost" size="sm" onClick={() => print(s)}>
                        <Printer className="size-3.5" />
                      </Button>
                      {s.status === "completed" && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => refund(s)}
                        >
                          <Undo2 className="size-3.5" />
                          {t("refunds")}
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
              {sales.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} className="h-20 text-center text-muted-foreground">
                    {t("noData")}
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}

function Stat({ title, value }: { title: string; value: string }) {
  return (
    <Card>
      <CardContent className="pt-5">
        <div className="text-sm text-muted-foreground">{title}</div>
        <div className="mt-1 text-2xl font-semibold tabular-nums">{value}</div>
      </CardContent>
    </Card>
  );
}

function KV({
  label,
  value,
  strong,
}: {
  label: string;
  value: string;
  strong?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-muted-foreground">{label}</span>
      <span className={strong ? "font-semibold tabular-nums" : "tabular-nums"}>
        {value}
      </span>
    </div>
  );
}
