import type { WorkflowDoc } from "@dataflow/shared";
import { Plus, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { useMemo, useState } from "react";

export function WorkflowList({
  workflows,
  onOpen,
  onCreate,
  creating,
  onToggleEnabled,
}: {
  workflows: WorkflowDoc[];
  onOpen: (id: string) => void;
  onCreate: () => void;
  creating?: boolean;
  onToggleEnabled: (id: string, enabled: boolean) => void;
}) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return workflows;
    return workflows.filter((w) => w.name.toLowerCase().includes(q));
  }, [workflows, query]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Your workflows and projects
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              className="h-9 w-56 pl-8"
              placeholder="Search workflows…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <Button disabled={creating} onClick={onCreate}>
            <Plus className="h-4 w-4" />
            Create workflow
          </Button>
        </div>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="p-6">
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="grid grid-cols-[1.5fr_100px_120px_140px] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <div>Name</div>
              <div>Status</div>
              <div>Version</div>
              <div>Updated</div>
            </div>
            {filtered.map((w) => (
              <div
                key={String(w._id)}
                className="grid w-full grid-cols-[1.5fr_100px_120px_140px] items-center gap-3 border-b px-4 py-3 text-sm last:border-b-0 hover:bg-muted/50"
              >
                <button
                  type="button"
                  onClick={() => onOpen(String(w._id))}
                  className="truncate text-left font-medium"
                >
                  {w.name}
                </button>
                <div>
                  <Switch
                    size="sm"
                    checked={w.enabled}
                    onCheckedChange={(checked) =>
                      onToggleEnabled(String(w._id), checked)
                    }
                    aria-label={
                      w.enabled ? "Disable workflow" : "Enable workflow"
                    }
                  />
                </div>
                <div className="text-muted-foreground">v{w.version}</div>
                <div className="truncate text-xs text-muted-foreground">
                  {new Date(w.updatedAt).toLocaleString()}
                </div>
              </div>
            ))}
            {!filtered.length ? (
              <div className="px-4 py-12 text-center text-sm text-muted-foreground">
                No workflows yet. Create one to open the editor.
              </div>
            ) : null}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
