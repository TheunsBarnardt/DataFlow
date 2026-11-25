import { useEffect, useState } from "react";
import type { WorkflowDoc } from "@dataflow/shared";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";

interface Run {
  _id: string;
  workflowName: string;
  status: string;
  trigger: string;
  startedAt: string;
  error?: string;
}

interface Variable {
  _id: string;
  key: string;
}

export function DashboardPanel({
  workflows,
  onOpenWorkflow,
  onNavigate,
  onToggleEnabled,
}: {
  workflows: WorkflowDoc[];
  onOpenWorkflow: (id: string) => void;
  onNavigate: (
    section: "workflows" | "variables" | "nodes" | "runs" | "insights" | "audit",
  ) => void;
  onToggleEnabled: (id: string, enabled: boolean) => void;
}) {
  const [runs, setRuns] = useState<Run[]>([]);
  const [variableCount, setVariableCount] = useState(0);
  const [nodeCount, setNodeCount] = useState(0);

  useEffect(() => {
    void Promise.all([
      api<{ runs: Run[] }>("/api/runs?limit=8"),
      api<{ variables: Variable[] }>("/api/variables"),
      api<{ nodes: unknown[] }>("/api/nodes"),
    ]).then(([r, v, n]) => {
      setRuns(r.runs);
      setVariableCount(v.variables.length);
      setNodeCount(n.nodes.length);
    });
  }, [workflows.length]);

  const enabled = workflows.filter((w) => w.enabled).length;
  const failed = runs.filter((r) => r.status === "failed").length;

  return (
    <ScrollArea className="h-full">
      <div className="flex flex-col gap-6 p-6">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
          <p className="text-sm text-muted-foreground">
            Overview of workflows, runs, variables, and available nodes.
          </p>
        </div>

        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {[
            { label: "Workflows", value: workflows.length, action: () => onNavigate("workflows") },
            { label: "Enabled", value: enabled, action: () => onNavigate("workflows") },
            { label: "Variables", value: variableCount, action: () => onNavigate("variables") },
            { label: "Node types", value: nodeCount, action: () => onNavigate("nodes") },
          ].map((card) => (
            <button
              key={card.label}
              type="button"
              onClick={card.action}
              className="rounded-lg border bg-card p-4 text-left transition-colors hover:bg-accent"
            >
              <div className="text-xs font-medium uppercase text-muted-foreground">
                {card.label}
              </div>
              <div className="mt-2 text-2xl font-semibold tabular-nums">{card.value}</div>
            </button>
          ))}
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
          <section className="rounded-lg border bg-card p-4">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="text-sm font-semibold">Recent workflows</h2>
              <Button variant="ghost" size="sm" onClick={() => onNavigate("workflows")}>
                Open
              </Button>
            </div>
            <div className="flex flex-col gap-2">
              {workflows.slice(0, 6).map((w) => (
                <div
                  key={String(w._id)}
                  className="flex items-center justify-between gap-2 rounded-md px-2 py-2 text-sm hover:bg-muted"
                >
                  <button
                    type="button"
                    onClick={() => onOpenWorkflow(String(w._id))}
                    className="min-w-0 flex-1 truncate text-left font-medium"
                  >
                    {w.name}
                  </button>
                  <Switch
                    size="sm"
                    checked={w.enabled}
                    onCheckedChange={(checked) =>
                      onToggleEnabled(String(w._id), checked)
                    }
                    aria-label={w.enabled ? "Disable workflow" : "Enable workflow"}
                  />
                </div>
              ))}
              {!workflows.length ? (
                <p className="px-2 text-sm text-muted-foreground">
                  No workflows yet — create one from Projects or the + button.
                </p>
              ) : null}
            </div>
          </section>

          <section className="rounded-lg border bg-card p-4">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="text-sm font-semibold">Recent runs</h2>
              <div className="flex items-center gap-2">
                {failed ? <Badge>{failed} failed</Badge> : null}
                <Button variant="ghost" size="sm" onClick={() => onNavigate("runs")}>
                  All runs
                </Button>
              </div>
            </div>
            <div className="flex flex-col gap-2">
              {runs.map((r) => (
                <div
                  key={r._id}
                  className="flex flex-col gap-1 rounded-md px-2 py-2 text-sm"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="min-w-0">
                      <div className="truncate font-medium">{r.workflowName}</div>
                      <div className="text-xs text-muted-foreground">
                        {r.trigger} · {new Date(r.startedAt).toLocaleString()}
                      </div>
                    </div>
                    <Badge
                      variant={
                        r.status === "failed"
                          ? "destructive"
                          : r.status === "completed"
                            ? "success"
                            : "secondary"
                      }
                    >
                      {r.status}
                    </Badge>
                  </div>
                  {r.error ? (
                    <div className="text-xs leading-snug text-destructive line-clamp-2">
                      {r.error}
                    </div>
                  ) : null}
                </div>
              ))}
              {!runs.length ? (
                <p className="px-2 text-sm text-muted-foreground">No runs yet.</p>
              ) : null}
            </div>
          </section>
        </div>
      </div>
    </ScrollArea>
  );
}
