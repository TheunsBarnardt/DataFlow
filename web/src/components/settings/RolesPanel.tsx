import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { ROLE_DEFINITIONS, type DataFlowRole } from "@/lib/roles";

type UserRow = {
  id: string;
  role?: string;
};

export function RolesPanel() {
  const [users, setUsers] = useState<UserRow[]>([]);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api<{ users: UserRow[] }>("/api/users");
      setUsers(res.users ?? []);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const u of users) {
      const role = u.role ?? "editor";
      map.set(role, (map.get(role) ?? 0) + 1);
    }
    return map;
  }, [users]);

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="border-b px-8 py-5">
        <h1 className="text-[1.75rem] font-normal leading-[1.25] tracking-normal">
          Roles
        </h1>
        <p className="mt-3 text-sm leading-[1.35] text-muted-foreground">
          Roles allow you to manage specific permissions tailored to your team&apos;s
          needs. Define granular access to workflows, credentials, and instance
          settings. Assign roles under People &amp; access → Users.
        </p>
      </div>

      <div className="grid gap-4 p-6 md:grid-cols-2 xl:grid-cols-3">
        {ROLE_DEFINITIONS.map((role) => {
          const count = counts.get(role.id) ?? 0;
          return (
            <div
              key={role.id}
              className="flex flex-col gap-3 rounded-lg border p-4"
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <div className="text-sm font-semibold">{role.name}</div>
                  <code className="text-[11px] text-muted-foreground">
                    {role.id}
                  </code>
                </div>
                <Badge variant="secondary" className="shrink-0">
                  {loading ? "…" : `${count} user${count === 1 ? "" : "s"}`}
                </Badge>
              </div>
              <p className="text-sm text-muted-foreground">{role.description}</p>
              <ul className="space-y-1.5 border-t pt-3">
                {role.capabilities.map((cap) => (
                  <li
                    key={cap.id}
                    className="flex items-center gap-2 text-[13px] text-foreground"
                  >
                    <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--brand)]" />
                    {cap.label}
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </div>

      <div className="border-t px-6 py-4 text-xs text-muted-foreground">
        Assign roles under{" "}
        <span className="font-medium text-foreground">People &amp; access → Users</span>
        {" "}
        (
        {(ROLE_DEFINITIONS.map((r) => r.id) as DataFlowRole[]).join(", ")}
        ).
      </div>
    </div>
  );
}
