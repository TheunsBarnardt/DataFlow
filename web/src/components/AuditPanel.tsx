import { useCallback, useEffect, useState } from "react";
import type { AuditEventDoc } from "@dataflow/shared";
import { Download, RefreshCw, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api } from "@/lib/api";

function actionVariant(
  action: AuditEventDoc["action"],
): "default" | "secondary" | "success" | "destructive" | "outline" {
  if (action === "workflow.deleted" || action === "run.failed") return "destructive";
  if (action === "run.completed") return "success";
  if (action.startsWith("run.")) return "outline";
  return "default";
}

function downloadJson(filename: string, data: unknown) {
  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function AuditPanel({
  refreshKey,
  actorName,
}: {
  refreshKey: number;
  actorName: string;
}) {
  const [events, setEvents] = useState<AuditEventDoc[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [tick, setTick] = useState(0);

  const load = useCallback(() => {
    setLoading(true);
    setError(null);
    void api<{ events: AuditEventDoc[] }>("/api/audit?limit=80")
      .then((r) => setEvents(r.events))
      .catch((e) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    load();
  }, [load, refreshKey, tick]);

  async function exportEvents() {
    if (exporting) return;
    setExporting(true);
    try {
      const r = await api<{ events: AuditEventDoc[] }>("/api/audit?limit=5000");
      const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
      downloadJson(`dataflow-audit-${stamp}.json`, {
        exportedAt: new Date().toISOString(),
        count: r.events.length,
        events: r.events,
      });
      toast.success(`Exported ${r.events.length} audit event${r.events.length === 1 ? "" : "s"}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Export failed");
    } finally {
      setExporting(false);
    }
  }

  async function clearEvents() {
    if (!events.length || clearing) return;
    const ok = window.confirm(
      "Clear the entire audit trail? This cannot be undone.",
    );
    if (!ok) return;
    setClearing(true);
    try {
      await api<{ ok: boolean; deletedCount: number }>("/api/audit", {
        method: "DELETE",
      });
      setEvents([]);
      toast.success("Audit trail cleared");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to clear audit trail");
    } finally {
      setClearing(false);
    }
  }

  return (
    <div className="flex h-full flex-col gap-3 p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">Audit trail</h2>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Who created, edited, or deleted a workflow — and when runs started or finished.
            Signed in as <span className="font-medium text-foreground">{actorName}</span>.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => void exportEvents()}
            disabled={exporting || (!events.length && !loading)}
          >
            <Download className="h-3.5 w-3.5" />
            Export
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="text-destructive hover:bg-destructive/10 hover:text-destructive"
            onClick={() => void clearEvents()}
            disabled={!events.length || clearing}
          >
            <Trash2 className="h-3.5 w-3.5" />
            Clear
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setTick((n) => n + 1)}
            disabled={loading}
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            Refresh
          </Button>
        </div>
      </div>

      {error ? <div className="text-sm text-destructive">{error}</div> : null}

      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-2">
          {events.map((e) => (
            <div key={e._id} className="rounded-md border bg-card p-2 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={actionVariant(e.action)}>{e.action}</Badge>
                <span className="font-medium">{e.resourceName ?? e.resourceId}</span>
                <Badge variant="outline">{e.resourceType}</Badge>
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">{e.actor}</span>
                {" · "}
                {new Date(e.at).toLocaleString()}
                {e.details && Object.keys(e.details).length ? (
                  <>
                    {" · "}
                    <span className="font-mono text-[10px]">
                      {JSON.stringify(e.details)}
                    </span>
                  </>
                ) : null}
              </div>
            </div>
          ))}
          {!events.length && !error && !loading ? (
            <div className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
              No audit events yet. Create, edit, or run a workflow to populate this trail.
            </div>
          ) : null}
          {loading && !events.length ? (
            <div className="text-sm text-muted-foreground">Loading audit events…</div>
          ) : null}
        </div>
      </ScrollArea>
    </div>
  );
}
