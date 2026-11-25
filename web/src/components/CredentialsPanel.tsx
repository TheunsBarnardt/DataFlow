import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CredentialDialog } from "@/components/CredentialDialog";
import { api } from "@/lib/api";
import { getCredentialType } from "@/lib/credentialTypes";
import {
  invalidateCredentialsCache,
  type CredentialItem,
} from "@/hooks/useCredentials";

export function CredentialsPanel() {
  const [items, setItems] = useState<CredentialItem[]>([]);
  const [query, setQuery] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);

  const reload = useCallback(async () => {
    invalidateCredentialsCache();
    const res = await api<{ credentials: CredentialItem[] }>("/api/credentials");
    setItems(res.credentials);
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter((c) => {
      const display =
        getCredentialType(c.type)?.displayName.toLowerCase() ?? "";
      return (
        c.name.toLowerCase().includes(q) ||
        c.type.toLowerCase().includes(q) ||
        display.includes(q)
      );
    });
  }, [items, query]);

  async function remove(id: string) {
    try {
      await api(`/api/credentials/${id}`, { method: "DELETE" });
      toast.success("Deleted");
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Credentials</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Secrets for nodes (Dataverse, SMTP, IMAP, …). Values are stored
            encrypted-at-rest and never shown in the list.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="w-56"
            placeholder="Search credentials..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Button onClick={() => setDialogOpen(true)}>Add credential</Button>
        </div>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="p-6">
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="grid grid-cols-[1.4fr_1fr_auto] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <div>Name</div>
              <div>Type</div>
              <div className="text-right">Actions</div>
            </div>

            {filtered.map((item) => {
              const def = getCredentialType(item.type);
              return (
                <div
                  key={item.id}
                  className="grid grid-cols-[1.4fr_1fr_auto] items-center gap-3 border-b px-4 py-3 last:border-b-0"
                >
                  <div className="truncate text-sm font-medium">{item.name}</div>
                  <div className="truncate text-sm text-muted-foreground">
                    {def?.displayName ?? item.type}
                  </div>
                  <div className="flex justify-end">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => void remove(item.id)}
                    >
                      Delete
                    </Button>
                  </div>
                </div>
              );
            })}

            {!filtered.length ? (
              <div className="px-4 py-10 text-center text-sm text-muted-foreground">
                No credentials yet. Click Add credential to create one.
              </div>
            ) : null}
          </div>
        </div>
      </ScrollArea>

      <CredentialDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onCreated={() => void reload()}
      />
    </div>
  );
}
