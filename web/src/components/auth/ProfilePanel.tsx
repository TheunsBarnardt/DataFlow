import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api";
import { authClient } from "@/lib/auth-client";

type MeUser = {
  id: string;
  name: string;
  email: string;
  username?: string | null;
  role?: string | null;
  companyName?: string | null;
};

export function ProfilePanel() {
  const [user, setUser] = useState<MeUser | null>(null);
  const [name, setName] = useState("");
  const [companyName, setCompanyName] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        const res = await api<{ user: MeUser }>("/api/me");
        setUser(res.user);
        setName(res.user.name ?? "");
        setCompanyName(res.user.companyName ?? "");
      } catch (err) {
        toast.error(err instanceof Error ? err.message : String(err));
      }
    })();
  }, []);

  async function saveProfile() {
    setBusy(true);
    try {
      const res = await api<{ user: MeUser }>("/api/me", {
        method: "PATCH",
        body: JSON.stringify({ name, companyName }),
      });
      setUser(res.user);
      toast.success("Profile updated");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function changePassword() {
    if (!currentPassword || !newPassword) {
      toast.error("Enter current and new password");
      return;
    }
    setBusy(true);
    try {
      const { error } = await authClient.changePassword({
        currentPassword,
        newPassword,
        revokeOtherSessions: true,
      });
      if (error) {
        toast.error(error.message ?? "Could not change password");
        return;
      }
      setCurrentPassword("");
      setNewPassword("");
      toast.success("Password changed");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  if (!user) {
    return (
      <div className="p-6 text-sm text-muted-foreground">Loading profile…</div>
    );
  }

  return (
    <div className="mx-auto flex h-full max-w-lg flex-col gap-6 overflow-auto p-6">
      <div>
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">Profile</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Account details and password
        </p>
      </div>

      <div className="space-y-3 rounded-lg border p-4">
        <div className="grid gap-1 text-sm">
          <span className="text-muted-foreground">Username</span>
          <span className="font-medium">{user.username ?? "—"}</span>
        </div>
        <div className="grid gap-1 text-sm">
          <span className="text-muted-foreground">Email</span>
          <span className="font-medium">{user.email}</span>
        </div>
        <div className="grid gap-1 text-sm">
          <span className="text-muted-foreground">Role</span>
          <span className="font-medium">{user.role ?? "—"}</span>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="profile-name" className="text-xs text-muted-foreground">
            Display name
          </Label>
          <Input
            id="profile-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label
            htmlFor="profile-company"
            className="text-xs text-muted-foreground"
          >
            Company
          </Label>
          <Input
            id="profile-company"
            value={companyName}
            onChange={(e) => setCompanyName(e.target.value)}
            placeholder="Optional"
          />
        </div>
        <Button type="button" onClick={() => void saveProfile()} disabled={busy}>
          Save profile
        </Button>
      </div>

      <div className="space-y-3 rounded-lg border p-4">
        <h2 className="text-sm font-semibold">Change password</h2>
        <div className="flex flex-col gap-1.5">
          <Label
            htmlFor="profile-current-password"
            className="text-xs text-muted-foreground"
          >
            Current password
          </Label>
          <Input
            id="profile-current-password"
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            autoComplete="current-password"
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label
            htmlFor="profile-new-password"
            className="text-xs text-muted-foreground"
          >
            New password
          </Label>
          <Input
            id="profile-new-password"
            type="password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            autoComplete="new-password"
          />
        </div>
        <Button
          type="button"
          variant="secondary"
          onClick={() => void changePassword()}
          disabled={busy}
        >
          Update password
        </Button>
      </div>
    </div>
  );
}
