import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import { DATAFLOW_ROLES } from "@/lib/roles";

type UserRow = {
  id: string;
  name: string;
  email: string;
  username?: string;
  role?: string;
  status?: string;
  companyName?: string;
  banned?: boolean;
};

export function UsersPanel() {
  const [users, setUsers] = useState<UserRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [email, setEmail] = useState("");
  const [username, setUsername] = useState("");
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("editor");
  const [companyName, setCompanyName] = useState("");
  const [busy, setBusy] = useState(false);

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

  async function createUser() {
    setBusy(true);
    try {
      await api("/api/users", {
        method: "POST",
        body: JSON.stringify({
          email,
          username,
          name,
          password,
          role,
          companyName,
        }),
      });
      toast.success("User created");
      setEmail("");
      setUsername("");
      setName("");
      setPassword("");
      setCompanyName("");
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function setUserRole(id: string, nextRole: string) {
    try {
      await api(`/api/users/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ role: nextRole }),
      });
      toast.success("Role updated");
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  async function setStatus(id: string, status: string) {
    try {
      await api(`/api/users/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ status }),
      });
      toast.success(status === "active" ? "User enabled" : "User disabled");
      await reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="border-b px-6 py-5">
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">Users</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Create accounts and manage roles (admin only)
        </p>
      </div>

      <div className="grid gap-6 p-6 lg:grid-cols-[minmax(280px,360px)_1fr]">
        <div className="space-y-3 rounded-lg border p-4">
          <h2 className="text-sm font-semibold">Create user</h2>
          <Input
            placeholder="Display name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Input
            placeholder="Username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
          <Input
            placeholder="Email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Input
            placeholder="Password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Input
            placeholder="Company (optional)"
            value={companyName}
            onChange={(e) => setCompanyName(e.target.value)}
          />
          <FormSelect
            value={role}
            options={DATAFLOW_ROLES.map((r) => ({ value: r, label: r }))}
            onChange={setRole}
          />
          <Button type="button" onClick={() => void createUser()} disabled={busy}>
            Create
          </Button>
        </div>

        <div className="min-w-0 overflow-auto rounded-lg border">
          {loading ? (
            <div className="p-4 text-sm text-muted-foreground">Loading…</div>
          ) : users.length === 0 ? (
            <div className="p-4 text-sm text-muted-foreground">No users</div>
          ) : (
            <table className="w-full text-left text-sm">
              <thead className="border-b bg-muted/40 text-xs text-muted-foreground">
                <tr>
                  <th className="px-3 py-2 font-medium">User</th>
                  <th className="px-3 py-2 font-medium">Role</th>
                  <th className="px-3 py-2 font-medium">Active</th>
                </tr>
              </thead>
              <tbody>
                {users.map((u) => {
                  const active =
                    (u.status ?? (u.banned ? "disabled" : "active")) ===
                    "active";
                  return (
                    <tr key={u.id} className="border-b last:border-0">
                      <td className="px-3 py-2">
                        <div className="font-medium">
                          {u.username || u.name}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {u.email}
                          {u.companyName ? ` · ${u.companyName}` : ""}
                        </div>
                      </td>
                      <td className="px-3 py-2">
                        <FormSelect
                          className="h-8 text-xs"
                          value={u.role ?? "editor"}
                          options={DATAFLOW_ROLES.map((r) => ({
                            value: r,
                            label: r,
                          }))}
                          onChange={(next) => void setUserRole(u.id, next)}
                        />
                      </td>
                      <td className="px-3 py-2">
                        <Switch
                          size="sm"
                          checked={active}
                          onCheckedChange={(checked) =>
                            void setStatus(u.id, checked ? "active" : "disabled")
                          }
                          aria-label={
                            active ? "Disable user" : "Enable user"
                          }
                        />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
