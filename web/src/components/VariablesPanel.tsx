import { useCallback, useEffect, useMemo, useState } from "react";
import type { VariableDoc } from "@dataflow/shared";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api } from "@/lib/api";
import { invalidateVariablesCache } from "@/hooks/useVariables";

type Variable = VariableDoc & { _id: string };

type Draft = {
  key: string;
  value: string;
  description?: string;
};

export function VariablesPanel() {
  const [items, setItems] = useState<Variable[]>([]);
  const [query, setQuery] = useState("");
  const [editingId, setEditingId] = useState<string | "new" | null>(null);
  const [draft, setDraft] = useState<Draft>({ key: "", value: "" });
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    invalidateVariablesCache();
    const res = await api<{ variables: Variable[] }>("/api/variables");
    setItems(res.variables);
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (v) =>
        v.key.toLowerCase().includes(q) ||
        v.value.toLowerCase().includes(q) ||
        (v.description ?? "").toLowerCase().includes(q),
    );
  }, [items, query]);

  function startNew() {
    setEditingId("new");
    setDraft({ key: "", value: "" });
  }

  function startEdit(item: Variable) {
    setEditingId(item._id);
    setDraft({ key: item.key, value: item.value, description: item.description });
  }

  function cancelEdit() {
    setEditingId(null);
    setDraft({ key: "", value: "" });
  }

  async function save() {
    const key = draft.key.trim();
    if (!key) {
      toast.error("Key is required");
      return;
    }
    setBusy(true);
    try {
      if (editingId === "new") {
        await api("/api/variables", {
          method: "POST",
          body: JSON.stringify({
            key,
            value: draft.value,
            type: "string",
            description: draft.description,
          }),
        });
        toast.success("Variable created");
      } else if (editingId) {
        await api(`/api/variables/${editingId}`, {
          method: "PUT",
          body: JSON.stringify({
            key,
            value: draft.value,
            type: "string",
            description: draft.description,
          }),
        });
        toast.success("Variable saved");
      }
      cancelEdit();
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    try {
      await api(`/api/variables/${id}`, { method: "DELETE" });
      toast.success("Deleted");
      if (editingId === id) cancelEdit();
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Variables</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Project variables usable in expressions as{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 text-xs">$vars.KEY</code>
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="w-56"
            placeholder="Search variables..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Button onClick={startNew} disabled={editingId === "new"}>
            Add variable
          </Button>
        </div>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="p-6">
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <div>Key</div>
              <div>Value</div>
              <div>Usage syntax</div>
              <div className="text-right">Actions</div>
            </div>

            {editingId === "new" ? (
              <div className="grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] items-center gap-3 border-b px-4 py-3">
                <Input
                  autoFocus
                  placeholder="Key"
                  value={draft.key}
                  onChange={(e) => setDraft((d) => ({ ...d, key: e.target.value }))}
                />
                <Input
                  placeholder="Value"
                  value={draft.value}
                  onChange={(e) => setDraft((d) => ({ ...d, value: e.target.value }))}
                />
                <code className="truncate text-sm text-muted-foreground">
                  $vars.{draft.key.trim() || "…"}
                </code>
                <div className="flex justify-end gap-2">
                  <Button variant="ghost" size="sm" onClick={cancelEdit}>
                    Cancel
                  </Button>
                  <Button size="sm" disabled={busy} onClick={() => void save()}>
                    Save
                  </Button>
                </div>
              </div>
            ) : null}

            {filtered.map((item) => {
              const isEditing = editingId === item._id;
              return (
                <div
                  key={item._id}
                  className="grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] items-center gap-3 border-b px-4 py-3 last:border-b-0"
                >
                  {isEditing ? (
                    <>
                      <Input
                        value={draft.key}
                        onChange={(e) => setDraft((d) => ({ ...d, key: e.target.value }))}
                      />
                      <Input
                        value={draft.value}
                        onChange={(e) => setDraft((d) => ({ ...d, value: e.target.value }))}
                      />
                      <code className="truncate text-sm text-muted-foreground">
                        $vars.{draft.key.trim() || item.key}
                      </code>
                      <div className="flex justify-end gap-2">
                        <Button variant="ghost" size="sm" onClick={cancelEdit}>
                          Cancel
                        </Button>
                        <Button size="sm" disabled={busy} onClick={() => void save()}>
                          Save
                        </Button>
                      </div>
                    </>
                  ) : (
                    <>
                      <div className="truncate text-sm font-medium">{item.key}</div>
                      <div className="truncate text-sm text-muted-foreground">{item.value}</div>
                      <code className="truncate text-sm text-muted-foreground">
                        $vars.{item.key}
                      </code>
                      <div className="flex justify-end gap-2">
                        <Button variant="outline" size="sm" onClick={() => startEdit(item)}>
                          Edit
                        </Button>
                        <Button variant="ghost" size="sm" onClick={() => void remove(item._id)}>
                          Delete
                        </Button>
                      </div>
                    </>
                  )}
                </div>
              );
            })}

            {!filtered.length && editingId !== "new" ? (
              <div className="px-4 py-10 text-center text-sm text-muted-foreground">
                No variables yet. Click Add variable to create one.
              </div>
            ) : null}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
