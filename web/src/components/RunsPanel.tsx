import { useEffect, useState } from "react";
import type { NodeRunLog, RunDoc } from "@dataflow/shared";
import { ChevronDownIcon, ChevronRightIcon, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { ScrollArea } from "@/components/ui/scroll-area";
import { BinaryViewer, hasBinaryData } from "@/components/BinaryViewer";
import { CodeEditor } from "@/components/CodeEditor";
import { api } from "@/lib/api";
import { formatDurationMs, logDurationMs } from "@/lib/runOverlay";

function JsonBlock({ value }: { value: unknown }) {
  if (value === undefined) {
    return <span className="text-muted-foreground">—</span>;
  }
  let text: string;
  try {
    text = JSON.stringify(value, null, 2);
  } catch {
    text = String(value);
  }
  return (
    <CodeEditor
      value={text}
      language="json"
      height={160}
      readOnly
      className="focus-within:ring-0"
      onChange={() => {}}
    />
  );
}

/** Prefer run.error; fall back to the first failed step message. */
function runErrorText(run: RunDoc): string | undefined {
  if (run.error?.trim()) return run.error.trim();
  const failed = (run.logs ?? []).find((l) => l.status === "failed" && l.error);
  return failed?.error?.trim() || undefined;
}

function StepRow({ log }: { log: NodeRunLog }) {
  const failed = log.status === "failed";
  const [open, setOpen] = useState(failed);
  return (
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className={
        failed
          ? "rounded border border-destructive/40 bg-destructive/5"
          : "rounded border bg-background/80"
      }
    >
      <CollapsibleTrigger className="flex w-full items-center gap-2 px-2 py-1.5 text-left text-xs">
        <span className="font-medium">{log.nodeName}</span>
        <Badge
          variant={
            failed
              ? "destructive"
              : log.status === "completed"
                ? "success"
                : "secondary"
          }
        >
          {log.status}
        </Badge>
        <span className="text-muted-foreground">
          {formatDurationMs(logDurationMs(log))}
        </span>
        <span className="ml-auto text-muted-foreground">
          {open ? (
            <ChevronDownIcon className="size-3.5" />
          ) : (
            <ChevronRightIcon className="size-3.5" />
          )}
        </span>
      </CollapsibleTrigger>
      {log.error && !open ? (
        <div className="border-t border-destructive/20 px-2 pb-1.5 text-[11px] leading-snug text-destructive line-clamp-2">
          {log.error}
        </div>
      ) : null}
      <CollapsibleContent>
        <div className="flex flex-col gap-2 border-t px-2 py-2">
          {log.error ? (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-2 py-1.5 text-xs text-destructive whitespace-pre-wrap break-words">
              {log.error}
            </div>
          ) : null}
          <div>
            <div className="mb-1 text-[10px] uppercase text-muted-foreground">
              Input
            </div>
            <JsonBlock value={log.input} />
          </div>
          <div>
            <div className="mb-1 text-[10px] uppercase text-muted-foreground">
              Output
            </div>
            {log.output != null && hasBinaryData(log.output) ? (
              <div className="mb-2">
                <BinaryViewer data={log.output} />
              </div>
            ) : null}
            <JsonBlock value={log.output} />
          </div>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

export function RunsPanel({
  refreshKey,
  workflowId,
  selectedRunId,
  onSelectRun,
  listOnly = false,
}: {
  refreshKey: number;
  workflowId?: string;
  /** Controlled selection (e.g. canvas overlay). */
  selectedRunId?: string | null;
  onSelectRun?: (run: RunDoc | null) => void;
  /** Hide the step-history pane (canvas shows status instead). */
  listOnly?: boolean;
}) {
  const [runs, setRuns] = useState<RunDoc[]>([]);
  const [internalSelectedId, setInternalSelectedId] = useState<string | null>(
    null,
  );
  const [detail, setDetail] = useState<RunDoc | null>(null);
  const [clearing, setClearing] = useState(false);
  const selectedId =
    selectedRunId !== undefined ? selectedRunId : internalSelectedId;

  useEffect(() => {
    const qs = new URLSearchParams({ limit: "40" });
    if (workflowId) qs.set("workflowId", workflowId);
    void api<{ runs: RunDoc[] }>(`/api/runs?${qs}`).then((r) => {
      setRuns(r.runs);
    });
  }, [refreshKey, workflowId]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    void api<{ run: RunDoc }>(`/api/runs/${selectedId}`).then((r) => {
      setDetail(r.run);
    });
  }, [selectedId, refreshKey]);

  function selectRun(id: string) {
    if (selectedRunId === undefined) setInternalSelectedId(id);
    void api<{ run: RunDoc }>(`/api/runs/${id}`).then((r) => {
      setDetail(r.run);
      onSelectRun?.(r.run);
    });
  }

  async function clearRuns() {
    if (!runs.length || clearing) return;
    setClearing(true);
    try {
      const qs = new URLSearchParams();
      if (workflowId) qs.set("workflowId", workflowId);
      const suffix = qs.toString() ? `?${qs}` : "";
      await api<{ ok: boolean }>(`/api/runs${suffix}`, { method: "DELETE" });
      setRuns([]);
      setDetail(null);
      if (selectedRunId === undefined) setInternalSelectedId(null);
      onSelectRun?.(null);
      toast.success("Runs cleared");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to clear runs");
    } finally {
      setClearing(false);
    }
  }

  const detailError = detail ? runErrorText(detail) : undefined;

  const list = (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">Runs</h2>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5 px-2 text-xs text-muted-foreground"
          disabled={!runs.length || clearing}
          onClick={() => void clearRuns()}
        >
          <Trash2 className="size-3.5" />
          Clear
        </Button>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-2">
          {runs.map((r) => {
            const err = runErrorText(r);
            const failed = r.status === "failed" || Boolean(err);
            return (
              <button
                key={r._id}
                type="button"
                onClick={() => selectRun(String(r._id))}
                className={
                  String(r._id) === selectedId
                    ? "rounded-md border bg-accent p-2 text-left text-sm"
                    : failed
                      ? "rounded-md border border-destructive/40 bg-destructive/5 p-2 text-left text-sm hover:bg-destructive/10"
                      : "rounded-md border bg-card p-2 text-left text-sm hover:bg-muted/50"
                }
              >
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium">{r.workflowName}</span>
                  <Badge
                    variant={
                      failed
                        ? "destructive"
                        : r.status === "completed"
                          ? "success"
                          : "secondary"
                    }
                  >
                    {r.status}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {r.trigger}
                  </span>
                </div>
                <div className="text-xs text-muted-foreground">
                  {r.startedAt}
                </div>
                {err ? (
                  <div className="mt-1 text-xs leading-snug text-destructive line-clamp-3 whitespace-pre-wrap break-words">
                    {err}
                  </div>
                ) : null}
              </button>
            );
          })}
          {!runs.length ? (
            <div className="text-sm text-muted-foreground">No runs yet</div>
          ) : null}
        </div>
      </ScrollArea>
    </div>
  );

  if (listOnly) {
    return <div className="h-full min-h-0 p-3">{list}</div>;
  }

  return (
    <ResizablePanelGroup
      orientation="horizontal"
      className="h-full min-h-0 p-3"
    >
      <ResizablePanel defaultSize="50%" minSize="25%" className="min-h-0">
        <div className="h-full min-h-0 pr-2">{list}</div>
      </ResizablePanel>

      <ResizableHandle withHandle />

      <ResizablePanel defaultSize="50%" minSize="25%" className="min-h-0">
        <div className="flex h-full min-h-0 flex-col gap-2 rounded-md border bg-card p-2">
          <h2 className="text-sm font-semibold">Step history</h2>
          {!detail ? (
            <div className="text-sm text-muted-foreground">
              Select a run to inspect each step&apos;s inputs, outputs, and
              failures.
            </div>
          ) : (
            <ScrollArea className="min-h-0 flex-1">
              <div className="flex flex-col gap-2 pr-2">
                <div className="text-xs text-muted-foreground">
                  {detail.workflowName} · {detail.status}
                  {detail.trigger ? ` · ${detail.trigger}` : ""}
                  {detail.actor ? ` · ${detail.actor}` : ""}
                  {detail.timeSavedMinutes
                    ? ` · saved ${detail.timeSavedMinutes} min`
                    : ""}
                </div>
                {detailError ? (
                  <div className="rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-xs text-destructive whitespace-pre-wrap break-words">
                    {detailError}
                  </div>
                ) : null}
                {(detail.logs ?? []).map((log) => (
                  <StepRow key={`${log.nodeId}-${log.startedAt}`} log={log} />
                ))}
                {!detail.logs?.length ? (
                  <div className="text-sm text-muted-foreground">
                    No step logs
                  </div>
                ) : null}
              </div>
            </ScrollArea>
          )}
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
