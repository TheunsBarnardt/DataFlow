import { useCallback, useEffect, useMemo, useState } from "react";
import type { InsightsByTime, InsightsSummary } from "@dataflow/shared";
import {
  ArrowDown,
  ArrowUp,
  ChevronRight,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

type MetricId =
  | "total"
  | "failed"
  | "failureRate"
  | "timeSaved"
  | "averageRunTime";

type SortKey =
  | "workflowName"
  | "total"
  | "failed"
  | "failureRate"
  | "timeSavedMinutes"
  | "averageRunTimeMs";

type TrendKind = "positive" | "negative" | "neutral";

function fmtCount(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}

function fmtRatio(n: number): string {
  return `${(n * 100).toFixed(1)}%`;
}

function fmtMs(n: number): string {
  if (n < 1000) return `${Math.round(n)}ms`;
  return `${(n / 1000).toFixed(2)}s`;
}

function fmtTimeSaved(minutes: number): string {
  if (minutes >= 60) {
    const hours = minutes / 60;
    return `${hours >= 100 ? Math.round(hours).toLocaleString("en-US") : hours.toFixed(hours >= 10 ? 0 : 1)}h`;
  }
  return `${Math.round(minutes)}m`;
}

function rangeLabel(days: number): string {
  return `Last ${days} days`;
}

function formatRange(from: string, to: string): string {
  const opts: Intl.DateTimeFormatOptions = {
    month: "short",
    day: "numeric",
    year: "numeric",
  };
  return `${new Date(from).toLocaleDateString("en-US", opts).replace(",", "")} – ${new Date(to).toLocaleDateString("en-US", opts)}`;
}

function metricValue(
  id: MetricId,
  insights: InsightsSummary,
): { display: string; raw: number; previous?: number } {
  switch (id) {
    case "total":
      return {
        display: fmtCount(insights.total.value),
        raw: insights.total.value,
        previous: insights.total.previous,
      };
    case "failed":
      return {
        display: fmtCount(insights.failed.value),
        raw: insights.failed.value,
        previous: insights.failed.previous,
      };
    case "failureRate":
      return {
        display: fmtRatio(insights.failureRate.value),
        raw: insights.failureRate.value,
        previous: insights.failureRate.previous,
      };
    case "timeSaved":
      return {
        display: fmtTimeSaved(insights.timeSaved.value),
        raw: insights.timeSaved.value,
        previous: insights.timeSaved.previous,
      };
    case "averageRunTime":
      return {
        display: fmtMs(insights.averageRunTime.value),
        raw: insights.averageRunTime.value,
        previous: insights.averageRunTime.previous,
      };
  }
}

function seriesValue(id: MetricId, point: InsightsByTime): number {
  switch (id) {
    case "total":
      return point.total;
    case "failed":
      return point.failed;
    case "failureRate":
      return point.failureRate * 100;
    case "timeSaved":
      return point.timeSavedMinutes / 60;
    case "averageRunTime":
      return point.averageRunTimeMs / 1000;
  }
}

/** Lower is better for failed / failureRate / averageRunTime */
function impactOf(id: MetricId): "higher-better" | "lower-better" | "neutral" {
  if (id === "failed" || id === "failureRate" || id === "averageRunTime") {
    return "lower-better";
  }
  if (id === "timeSaved" || id === "total") return "higher-better";
  return "neutral";
}

function trendFor(
  id: MetricId,
  current: number,
  previous?: number,
): { text: string; kind: TrendKind; dir: "up" | "down" | "flat" } | null {
  if (previous === undefined) return null;
  const d = current - previous;
  if (Math.abs(d) < 1e-9) {
    return { text: "±0", kind: "neutral", dir: "flat" };
  }
  const dir = d > 0 ? "up" : "down";
  let text: string;
  if (id === "failureRate") {
    text = `${d > 0 ? "+" : ""}${(d * 100).toFixed(1)}pp`;
  } else if (id === "timeSaved") {
    const mins = Math.abs(d);
    text = `${d > 0 ? "+" : "−"}${fmtTimeSaved(mins)}`;
  } else if (id === "averageRunTime") {
    text = `${d > 0 ? "+" : "−"}${fmtMs(Math.abs(d))}`;
  } else if (id === "total" || id === "failed") {
    const pct = previous ? (d / previous) * 100 : 0;
    text = `${Math.abs(pct).toFixed(1)}%`;
  } else {
    text = String(d);
  }

  const impact = impactOf(id);
  let kind: TrendKind = "neutral";
  if (impact === "higher-better") kind = d > 0 ? "positive" : "negative";
  if (impact === "lower-better") kind = d < 0 ? "positive" : "negative";
  return { text, kind, dir };
}

const METRICS: {
  id: MetricId;
  label: string;
  chartUnit: string;
}[] = [
  { id: "total", label: "Prod. executions", chartUnit: "day" },
  { id: "failed", label: "Failed prod. executions", chartUnit: "day" },
  { id: "failureRate", label: "Failure rate", chartUnit: "%" },
  { id: "timeSaved", label: "Time saved", chartUnit: "h" },
  { id: "averageRunTime", label: "Run time (avg.)", chartUnit: "s" },
];

function AreaChart({
  data,
  metric,
}: {
  data: InsightsByTime[];
  metric: MetricId;
}) {
  const width = 720;
  const height = 220;
  const pad = { top: 16, right: 12, bottom: 28, left: 44 };
  const values = data.map((d) => seriesValue(metric, d));
  const max = Math.max(...values, 0);
  const niceMax = max === 0 ? 1 : max * 1.1;
  const innerW = width - pad.left - pad.right;
  const innerH = height - pad.top - pad.bottom;

  const points = data.map((d, i) => {
    const x =
      pad.left +
      (data.length <= 1 ? innerW / 2 : (i / (data.length - 1)) * innerW);
    const y = pad.top + innerH - (seriesValue(metric, d) / niceMax) * innerH;
    return { x, y, label: d.date };
  });

  const line =
    points.length === 0
      ? ""
      : points
          .map((p, i) => `${i === 0 ? "M" : "L"}${p.x.toFixed(1)},${p.y.toFixed(1)}`)
          .join(" ");
  const area =
    points.length === 0
      ? ""
      : `${line} L${points[points.length - 1]!.x.toFixed(1)},${(pad.top + innerH).toFixed(1)} L${points[0]!.x.toFixed(1)},${(pad.top + innerH).toFixed(1)} Z`;

  const ticks = [0, 0.5, 1].map((t) => niceMax * t);
  const midLabel =
    METRICS.find((m) => m.id === metric)?.chartUnit === "day"
      ? "day"
      : METRICS.find((m) => m.id === metric)?.chartUnit ?? "";

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="h-[220px] w-full"
      role="img"
      aria-label="Insights trend"
    >
      <defs>
        <linearGradient id="insightsFill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.35" />
          <stop offset="100%" stopColor="var(--primary)" stopOpacity="0.02" />
        </linearGradient>
      </defs>
      {ticks.map((v, i) => {
        const y = pad.top + innerH - (v / niceMax) * innerH;
        return (
          <g key={i}>
            <line
              x1={pad.left}
              x2={width - pad.right}
              y1={y}
              y2={y}
              stroke="var(--border)"
              strokeDasharray="3 4"
            />
            <text
              x={pad.left - 8}
              y={y + 3}
              textAnchor="end"
              className="fill-muted-foreground"
              fontSize="10"
            >
              {metric === "failureRate"
                ? v.toFixed(1)
                : metric === "averageRunTime"
                  ? v.toFixed(2)
                  : Math.round(v).toLocaleString("en-US")}
            </text>
          </g>
        );
      })}
      <text
        x={12}
        y={pad.top + innerH / 2}
        transform={`rotate(-90 12 ${pad.top + innerH / 2})`}
        className="fill-muted-foreground"
        fontSize="10"
        textAnchor="middle"
      >
        {midLabel}
      </text>
      {area ? <path d={area} fill="url(#insightsFill)" /> : null}
      {line ? (
        <path
          d={line}
          fill="none"
          stroke="var(--primary)"
          strokeWidth="2"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      ) : null}
      {points.map((p, i) => (
        <text
          key={p.label}
          x={p.x}
          y={height - 8}
          textAnchor="middle"
          className="fill-muted-foreground"
          fontSize="10"
        >
          {i === 0 || i === points.length - 1 || i % Math.ceil(points.length / 6) === 0
            ? new Date(p.label + "T00:00:00Z").toLocaleDateString("en-US", {
                month: "short",
                day: "numeric",
              })
            : ""}
        </text>
      ))}
    </svg>
  );
}

export function InsightsPanel({ refreshKey }: { refreshKey: number }) {
  const [insights, setInsights] = useState<InsightsSummary | null>(null);
  const [days, setDays] = useState(7);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);
  const [selected, setSelected] = useState<MetricId>("timeSaved");
  const [sortKey, setSortKey] = useState<SortKey>("timeSavedMinutes");
  const [sortDesc, setSortDesc] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    void api<{ insights: InsightsSummary }>(`/api/insights?days=${days}`)
      .then((r) => setInsights(r.insights))
      .catch((e) => {
        setInsights(null);
        const msg = e instanceof Error ? e.message : String(e);
        setError(
          msg === "Not found" || msg === "Failed to fetch"
            ? "Insights API unavailable. Start the DataFlow server (npm run dev)."
            : msg,
        );
      })
      .finally(() => setLoading(false));
  }, [days]);

  useEffect(() => {
    load();
  }, [load, refreshKey, tick]);

  const rows = useMemo(() => {
    const list = [...(insights?.byWorkflow ?? [])];
    list.sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (typeof av === "string" && typeof bv === "string") {
        return sortDesc ? bv.localeCompare(av) : av.localeCompare(bv);
      }
      return sortDesc
        ? Number(bv) - Number(av)
        : Number(av) - Number(bv);
    });
    return list;
  }, [insights, sortKey, sortDesc]);

  function toggleSort(key: SortKey) {
    if (sortKey === key) setSortDesc((d) => !d);
    else {
      setSortKey(key);
      setSortDesc(true);
    }
  }

  const selectedMeta = METRICS.find((m) => m.id === selected)!;

  return (
    <div className="flex h-full flex-col gap-3 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <h2 className="text-base font-semibold tracking-tight">Insights</h2>
        <div className="flex-1" />
        {insights ? (
          <span className="rounded-md border bg-card px-2.5 py-1 text-xs text-muted-foreground">
            {formatRange(insights.from, insights.to)}
          </span>
        ) : null}
        <div className="flex overflow-hidden rounded-md border">
          {[7, 14, 30].map((d) => (
            <button
              key={d}
              type="button"
              onClick={() => setDays(d)}
              className={cn(
                "px-2.5 py-1 text-xs transition-colors",
                days === d
                  ? "bg-primary text-primary-foreground"
                  : "bg-card text-muted-foreground hover:bg-muted",
              )}
            >
              {d}d
            </button>
          ))}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => setTick((n) => n + 1)}
          disabled={loading}
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      <div className="grid grid-cols-2 gap-2 lg:grid-cols-5">
        {METRICS.map((m) => {
          const values = insights ? metricValue(m.id, insights) : null;
          const trend =
            values != null
              ? trendFor(m.id, values.raw, values.previous)
              : null;
          const active = selected === m.id;
          return (
            <button
              key={m.id}
              type="button"
              onClick={() => {
                setSelected(m.id);
                const map: Record<MetricId, SortKey> = {
                  total: "total",
                  failed: "failed",
                  failureRate: "failureRate",
                  timeSaved: "timeSavedMinutes",
                  averageRunTime: "averageRunTimeMs",
                };
                setSortKey(map[m.id]);
                setSortDesc(true);
              }}
              className={cn(
                "rounded-md border bg-card px-3 py-2.5 text-left transition-colors hover:bg-accent/40",
                active && "border-primary ring-1 ring-primary/40",
              )}
            >
              <div className="text-xs font-medium text-foreground">{m.label}</div>
              <div className="text-[10px] text-muted-foreground">
                {rangeLabel(days)}
              </div>
              <div className="mt-2 flex items-end justify-between gap-2">
                <div className="text-xl font-semibold tabular-nums tracking-tight">
                  {loading && !insights ? "…" : (values?.display ?? "—")}
                </div>
                {trend ? (
                  <span
                    className={cn(
                      "inline-flex items-center gap-0.5 text-[11px] tabular-nums",
                      trend.kind === "positive" && "text-primary",
                      trend.kind === "negative" && "text-destructive",
                      trend.kind === "neutral" && "text-muted-foreground",
                    )}
                  >
                    {trend.dir === "up" ? (
                      <ArrowUp className="h-3 w-3" />
                    ) : trend.dir === "down" ? (
                      <ArrowDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    {trend.text}
                  </span>
                ) : null}
              </div>
              <div
                className={cn(
                  "mt-2 h-0.5 rounded-full",
                  active ? "bg-primary" : "bg-transparent",
                )}
              />
            </button>
          );
        })}
      </div>

      <div className="min-h-0 flex-1 overflow-hidden rounded-md border bg-card">
        <ScrollArea className="h-full">
          <div className="flex flex-col gap-4 p-3">
            <div className="rounded-md border bg-background/40 p-2">
              {insights?.byTime?.length ? (
                <AreaChart data={insights.byTime} metric={selected} />
              ) : (
                <div className="flex h-[220px] items-center justify-center text-sm text-muted-foreground">
                  {loading
                    ? "Loading chart…"
                    : "No execution data in this window"}
                </div>
              )}
            </div>

            <div>
              <h3 className="mb-2 text-sm font-semibold">
                Breakdown by workflow
              </h3>
              <div className="overflow-x-auto rounded-md border">
                <table className="w-full min-w-[720px] text-left text-sm">
                  <thead className="border-b bg-muted/40 text-xs text-muted-foreground">
                    <tr>
                      {(
                        [
                          ["workflowName", "Name"],
                          ["total", "Prod. executions"],
                          ["failed", "Failed prod. executions"],
                          ["failureRate", "Failure rate"],
                          ["timeSavedMinutes", "Time saved"],
                          ["averageRunTimeMs", "Run time (avg.)"],
                        ] as [SortKey, string][]
                      ).map(([key, label]) => (
                        <th key={key} className="px-3 py-2 font-medium">
                          <button
                            type="button"
                            className="inline-flex items-center gap-1 hover:text-foreground"
                            onClick={() => toggleSort(key)}
                          >
                            {label}
                            {sortKey === key ? (
                              sortDesc ? (
                                <ArrowDown className="h-3 w-3" />
                              ) : (
                                <ArrowUp className="h-3 w-3" />
                              )
                            ) : null}
                          </button>
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((w) => (
                      <tr
                        key={w.workflowId}
                        className="border-b last:border-0 hover:bg-muted/30"
                      >
                        <td className="px-3 py-2.5">
                          <span className="font-medium underline-offset-2 hover:underline">
                            {w.workflowName}
                          </span>
                        </td>
                        <td className="px-3 py-2.5 tabular-nums">
                          {fmtCount(w.total)}
                        </td>
                        <td className="px-3 py-2.5 tabular-nums">
                          {fmtCount(w.failed)}
                        </td>
                        <td className="px-3 py-2.5 tabular-nums">
                          {fmtRatio(w.failureRate)}
                        </td>
                        <td className="px-3 py-2.5 tabular-nums">
                          {fmtTimeSaved(w.timeSavedMinutes)}
                        </td>
                        <td className="px-3 py-2.5 tabular-nums">
                          {fmtMs(w.averageRunTimeMs)}
                        </td>
                      </tr>
                    ))}
                    {!loading && insights && rows.length === 0 ? (
                      <tr>
                        <td
                          colSpan={6}
                          className="px-3 py-8 text-center text-muted-foreground"
                        >
                          No executions in this window. Run a workflow to see
                          metrics for {selectedMeta.label.toLowerCase()}.
                        </td>
                      </tr>
                    ) : null}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
