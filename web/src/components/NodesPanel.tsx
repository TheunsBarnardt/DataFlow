import { useEffect, useMemo, useState } from "react";
import type { NodeType } from "@dataflow/shared";
import { Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { getNodeSchema } from "@/lib/nodeSchemas";
import { getNodeColor, getNodeIcon } from "@/lib/nodeIcons";
import { cn } from "@/lib/utils";

export interface NodeCatalogItem {
  type: NodeType;
  label: string;
  category: string;
}

type KindFilter = "all" | "triggers" | "actions";

const RECENT_KEY = "dataflow.recentNodes";
const RECENT_LIMIT = 8;

function readRecentTypes(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((t): t is string => typeof t === "string");
  } catch {
    return [];
  }
}

function pushRecentType(type: string) {
  const next = [type, ...readRecentTypes().filter((t) => t !== type)].slice(
    0,
    RECENT_LIMIT,
  );
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    /* ignore quota */
  }
  return next;
}

export function NodesPanel({
  onAddNode,
  canAdd,
  compact = false,
  hideHeader = false,
  triggersOnly = false,
  kindFilter: kindFilterProp,
  variant = "list",
}: {
  onAddNode?: (item: NodeCatalogItem) => void;
  canAdd?: boolean;
  compact?: boolean;
  hideHeader?: boolean;
  /** When true, lock filter to triggers (Add first step). */
  triggersOnly?: boolean;
  kindFilter?: KindFilter;
  /** list = classic rows; grid = icon-grid node creator */
  variant?: "list" | "grid";
}) {
  const [nodes, setNodes] = useState<NodeCatalogItem[]>([]);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string>("all");
  const [kind, setKind] = useState<KindFilter>(
    triggersOnly ? "triggers" : (kindFilterProp ?? "all"),
  );
  const [recentTypes, setRecentTypes] = useState<string[]>(() =>
    typeof window !== "undefined" ? readRecentTypes() : [],
  );

  useEffect(() => {
    if (triggersOnly) setKind("triggers");
    else if (kindFilterProp) setKind(kindFilterProp);
  }, [triggersOnly, kindFilterProp]);

  useEffect(() => {
    void api<{ nodes: NodeCatalogItem[] }>("/api/nodes").then((r) =>
      setNodes(r.nodes),
    );
  }, []);

  const categories = useMemo(() => {
    const set = new Set(nodes.map((n) => n.category));
    return ["all", ...Array.from(set).sort()];
  }, [nodes]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return nodes.filter((n) => {
      if (kind === "triggers" && n.category !== "trigger") return false;
      if (kind === "actions" && n.category === "trigger") return false;
      if (category !== "all" && n.category !== category) return false;
      if (!q) return true;
      return (
        n.label.toLowerCase().includes(q) ||
        n.type.toLowerCase().includes(q) ||
        n.category.toLowerCase().includes(q)
      );
    });
  }, [nodes, query, category, kind]);

  const grouped = useMemo(() => {
    const map = new Map<string, NodeCatalogItem[]>();
    for (const n of filtered) {
      const list = map.get(n.category) ?? [];
      list.push(n);
      map.set(n.category, list);
    }
    return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [filtered]);

  const recentItems = useMemo(() => {
    const byType = new Map(nodes.map((n) => [n.type, n]));
    return recentTypes
      .map((t) => byType.get(t as NodeType))
      .filter((n): n is NodeCatalogItem => Boolean(n))
      .filter((n) => {
        if (kind === "triggers" && n.category !== "trigger") return false;
        if (kind === "actions" && n.category === "trigger") return false;
        return true;
      });
  }, [nodes, recentTypes, kind]);

  function handleAdd(item: NodeCatalogItem) {
    if (!onAddNode) return;
    if (canAdd === false) return;
    setRecentTypes(pushRecentType(item.type));
    onAddNode(item);
  }

  const filters = (
    <div
      className={`flex flex-col gap-2 border-b ${compact || variant === "grid" ? "p-2" : "px-6 py-3"}`}
    >
      <Input
        placeholder="Search nodes…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <div className="flex items-center gap-1 rounded-md bg-muted/50 p-0.5">
        {(
          [
            ["all", "All"],
            ["triggers", "Triggers"],
            ["actions", "Actions"],
          ] as const
        ).map(([k, label]) => (
          <button
            key={k}
            type="button"
            disabled={triggersOnly && k !== "triggers"}
            onClick={() => setKind(k)}
            className={cn(
              "flex flex-1 items-center justify-center gap-1 rounded-md px-2 py-1.5 text-xs font-medium",
              kind === k
                ? "bg-card shadow-sm"
                : "text-muted-foreground hover:text-foreground",
              triggersOnly && k !== "triggers" && "opacity-40",
            )}
          >
            {k === "triggers" ? (
              <Zap className="h-3 w-3 text-amber-500" />
            ) : null}
            {label}
          </button>
        ))}
      </div>
      {variant === "list" ? (
        <div className="flex flex-wrap gap-1">
          {categories.map((c) => (
            <button
              key={c}
              type="button"
              onClick={() => setCategory(c)}
              className={
                category === c
                  ? "rounded-md bg-accent px-2 py-1 text-xs"
                  : "rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-muted"
              }
            >
              {c}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );

  return (
    <div className="flex h-full flex-col">
      {!hideHeader ? (
        !compact ? (
          <div className="border-b px-6 py-4">
            <h1 className="text-xl font-semibold tracking-tight">Nodes</h1>
            <p className="text-sm text-muted-foreground">
              Browse node types. Open a workflow, then add a node to the canvas.
            </p>
          </div>
        ) : (
          <div className="border-b px-3 py-2 text-xs font-medium uppercase text-muted-foreground">
            Nodes
          </div>
        )
      ) : null}

      {filters}

      <ScrollArea className="min-h-0 flex-1">
        {variant === "grid" ? (
          <div className="flex flex-col gap-4 p-3">
            {recentItems.length > 0 && !query.trim() ? (
              <section>
                <div className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Recently used
                </div>
                <NodeIconGrid
                  items={recentItems}
                  canAdd={canAdd !== false}
                  onAdd={handleAdd}
                />
              </section>
            ) : null}

            {grouped.map(([cat, items]) => (
              <section key={cat}>
                <div className="mb-2 flex items-center gap-2">
                  <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {cat}
                  </h2>
                  <Badge>{items.length}</Badge>
                </div>
                <NodeIconGrid
                  items={items}
                  canAdd={canAdd !== false}
                  onAdd={handleAdd}
                />
              </section>
            ))}
            {!filtered.length ? (
              <p className="text-sm text-muted-foreground">No nodes match.</p>
            ) : null}
          </div>
        ) : (
          <div className={`flex flex-col gap-4 ${compact ? "p-2" : "p-6"}`}>
            {grouped.map(([cat, items]) => (
              <section key={cat}>
                <div className="mb-2 flex items-center gap-2">
                  <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    {cat}
                  </h2>
                  <Badge>{items.length}</Badge>
                </div>
                <div className="flex flex-col gap-1">
                  {items.map((item) => {
                    const schema = getNodeSchema(item.type);
                    return (
                      <div
                        key={item.type}
                        className="flex items-start gap-2 rounded-md border bg-card px-2 py-2"
                      >
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-1.5 truncate text-sm font-medium">
                            {item.category === "trigger" ? (
                              <Zap className="h-3.5 w-3.5 shrink-0 text-amber-500" />
                            ) : null}
                            {item.label}
                          </div>
                          <div className="truncate text-xs text-muted-foreground">
                            {item.type}
                          </div>
                          <p className="mt-1 line-clamp-2 text-[11px] leading-snug text-muted-foreground">
                            {schema.description}
                          </p>
                        </div>
                        {onAddNode ? (
                          <Button
                            size="sm"
                            variant="outline"
                            className="shrink-0"
                            disabled={!canAdd}
                            onClick={() => handleAdd(item)}
                          >
                            Add
                          </Button>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              </section>
            ))}
            {!filtered.length ? (
              <p className="text-sm text-muted-foreground">No nodes match.</p>
            ) : null}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

function NodeIconGrid({
  items,
  canAdd,
  onAdd,
}: {
  items: NodeCatalogItem[];
  canAdd: boolean;
  onAdd: (item: NodeCatalogItem) => void;
}) {
  return (
    <div className="grid grid-cols-3 gap-2 sm:grid-cols-4">
      {items.map((item) => {
        const Icon = getNodeIcon(item.type);
        const color = getNodeColor(item.type);
        return (
          <button
            key={item.type}
            type="button"
            disabled={!canAdd}
            title={`${item.label} (${item.type})`}
            onClick={() => onAdd(item)}
            className={cn(
              "flex flex-col items-center gap-1.5 rounded-lg border bg-card px-2 py-3 text-center transition-colors",
              "hover:border-foreground/20 hover:bg-muted/40",
              "disabled:pointer-events-none disabled:opacity-50",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            )}
          >
            <span
              className="flex h-10 w-10 items-center justify-center rounded-xl text-white shadow-sm"
              style={{ background: color }}
            >
              <Icon className="h-5 w-5" strokeWidth={2} />
            </span>
            <span className="line-clamp-2 w-full text-[11px] font-medium leading-tight">
              {item.label}
            </span>
          </button>
        );
      })}
    </div>
  );
}
