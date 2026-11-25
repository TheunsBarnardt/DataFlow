import { useCallback, useEffect, useState } from "react";
import type { WorkflowDoc } from "@dataflow/shared";
import { Toaster } from "sonner";
import { toast } from "sonner";
import { RunsPanel } from "@/components/RunsPanel";
import { InsightsPanel } from "@/components/InsightsPanel";
import { AuditPanel } from "@/components/AuditPanel";
import { DashboardPanel } from "@/components/DashboardPanel";
import { VariablesPanel } from "@/components/VariablesPanel";
import { CredentialsPanel } from "@/components/CredentialsPanel";
import { NodesPanel, type NodeCatalogItem } from "@/components/NodesPanel";
import { WorkflowList } from "@/components/WorkflowList";
import { ApiProjectsPanel } from "@/components/ApiProjectsPanel";
import { LoginPage } from "@/components/auth/LoginPage";
import { SettingsPanel } from "@/components/settings/SettingsPanel";
import { SettingsSidebar } from "@/components/settings/SettingsSidebar";
import type { SettingsPageId } from "@/components/settings/settingsNav";
import { MainSidebar, type NavSection } from "@/components/layout/MainSidebar";
import { WorkflowEditor } from "@/components/layout/WorkflowEditor";
import type { PendingNodeAdd } from "@/components/FlowCanvas";
import { api } from "@/lib/api";
import { authClient } from "@/lib/auth-client";
import { applyTheme, initTheme, type Theme } from "@/lib/theme";

type SessionUser = {
  id: string;
  name: string;
  email: string;
  username?: string | null;
  role?: string | null;
};

export default function App() {
  const [theme, setTheme] = useState<Theme>(() => initTheme());
  const [authLoading, setAuthLoading] = useState(true);
  const [sessionUser, setSessionUser] = useState<SessionUser | null>(null);
  const [section, setSection] = useState<NavSection>("dashboard");
  const [settingsPage, setSettingsPage] =
    useState<SettingsPageId>("profile");
  const [collapsed, setCollapsed] = useState(false);
  const [workflows, setWorkflows] = useState<WorkflowDoc[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [apiProjectId, setApiProjectId] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [agentOpen, setAgentOpen] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [pendingAdd, setPendingAdd] = useState<PendingNodeAdd | null>(null);
  const [creating, setCreating] = useState(false);

  const actorName =
    sessionUser?.username || sessionUser?.name || sessionUser?.email || "User";

  const refreshSession = useCallback(async () => {
    try {
      const { data } = await authClient.getSession();
      if (data?.user) {
        const u = data.user as SessionUser;
        setSessionUser({
          id: u.id,
          name: u.name,
          email: u.email,
          username: (u as { username?: string }).username ?? null,
          role: (u as { role?: string }).role ?? null,
        });
        return true;
      }
      setSessionUser(null);
      return false;
    } catch {
      setSessionUser(null);
      return false;
    }
  }, []);

  useEffect(() => {
    void (async () => {
      await refreshSession();
      setAuthLoading(false);
    })();
  }, [refreshSession]);

  const reload = useCallback(async () => {
    const res = await api<{ workflows: WorkflowDoc[] }>("/api/workflows");
    setWorkflows(res.workflows);
    setRefreshKey((k) => k + 1);
    setSelectedId((prev) => {
      if (prev && res.workflows.some((w) => String(w._id) === prev)) return prev;
      return res.workflows[0]?._id ? String(res.workflows[0]._id) : null;
    });
  }, []);

  useEffect(() => {
    if (!sessionUser) return;
    void reload();
  }, [reload, sessionUser]);

  const selected =
    workflows.find((w) => String(w._id) === selectedId) ?? null;

  function toggleTheme() {
    const next: Theme = theme === "dark" ? "light" : "dark";
    applyTheme(next);
    setTheme(next);
  }

  async function logout() {
    await authClient.signOut();
    setSessionUser(null);
    setWorkflows([]);
    setSection("dashboard");
    setEditorOpen(false);
    toast.success("Signed out");
  }

  async function createWorkflow() {
    setCreating(true);
    try {
      const res = await api<{ workflow: WorkflowDoc }>("/api/workflows", {
        method: "POST",
        body: JSON.stringify({ name: "My workflow" }),
      });
      await reload();
      setSelectedId(String(res.workflow._id));
      setSection("workflows");
      setEditorOpen(true);
      toast.success("Workflow created");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }

  async function toggleWorkflowEnabled(id: string, enabled: boolean) {
    try {
      await api(`/api/workflows/${id}`, {
        method: "PUT",
        body: JSON.stringify({ enabled }),
      });
      setWorkflows((prev) =>
        prev.map((w) =>
          String(w._id) === id ? { ...w, enabled } : w,
        ),
      );
      toast.success(enabled ? "Workflow enabled" : "Workflow disabled");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  function openWorkflow(id: string) {
    const wf = workflows.find((w) => String(w._id) === id);
    const isApi = wf?.kind === "apiEndpoint" || Boolean(wf?.apiProjectId);
    setSelectedId(id);
    if (isApi && wf?.apiProjectId) {
      setApiProjectId(String(wf.apiProjectId));
    }
    setSection((prev) => {
      if (prev === "apis" || isApi) return "apis";
      return "workflows";
    });
    setEditorOpen(true);
    void reload().then(() => setSelectedId(id));
  }

  function handleAddNode(
    item: NodeCatalogItem,
    opts?: { connectFrom?: string; sourceHandle?: string },
  ) {
    if (!selectedId || !editorOpen) {
      toast.error("Open a workflow first");
      setSection("workflows");
      setEditorOpen(false);
      return;
    }
    setPendingAdd({
      type: item.type,
      label: item.label,
      token: Date.now(),
      connectFrom: opts?.connectFrom,
      sourceHandle: opts?.sourceHandle,
    });
    setSection((prev) => (prev === "apis" ? "apis" : "workflows"));
  }

  function backToList() {
    const isApi =
      selected?.kind === "apiEndpoint" || Boolean(selected?.apiProjectId);
    setEditorOpen(false);
    setAgentOpen(false);
    if (isApi) {
      setSection("apis");
      if (selected?.apiProjectId) {
        setApiProjectId(String(selected.apiProjectId));
      }
    }
  }

  function navigate(s: NavSection) {
    setSection(s);
    if (s !== "workflows" && s !== "apis") {
      setEditorOpen(false);
      setAgentOpen(false);
    }
  }

  function openSettings(page: SettingsPageId = "profile") {
    setSettingsPage(page);
    setSection("settings");
    setEditorOpen(false);
    setAgentOpen(false);
  }

  const showEditor =
    (section === "workflows" || section === "apis") &&
    editorOpen &&
    selected;

  if (authLoading) {
    return (
      <div className="flex h-full items-center justify-center bg-background text-sm text-muted-foreground">
        Loading…
        <Toaster theme={theme} />
      </div>
    );
  }

  if (!sessionUser) {
    return (
      <>
        <LoginPage
          onLoggedIn={() => {
            void refreshSession();
          }}
        />
        <Toaster theme={theme} />
      </>
    );
  }

  const inSettings = section === "settings";

  return (
    <div className="flex h-full bg-background text-foreground">
      {inSettings ? (
        <SettingsSidebar
          page={settingsPage}
          isAdmin={sessionUser.role === "admin"}
          onSelect={setSettingsPage}
          onBack={() => navigate("dashboard")}
        />
      ) : (
        <MainSidebar
          section={section}
          onNavigate={navigate}
          onNewWorkflow={() => void createWorkflow()}
          collapsed={collapsed}
          onToggleCollapse={() => setCollapsed((v) => !v)}
          theme={theme}
          onToggleTheme={toggleTheme}
          actorName={actorName}
          role={sessionUser.role ?? undefined}
          onProfile={() => openSettings("profile")}
          onOpenSettings={openSettings}
          onLogout={() => void logout()}
        />
      )}

      <div className="flex min-w-0 flex-1 flex-col">
        {showEditor ? (
          <WorkflowEditor
            workflow={selected}
            theme={theme}
            pendingAdd={pendingAdd}
            refreshKey={refreshKey}
            onSaved={() => void reload()}
            onAddNode={handleAddNode}
            onBackToList={backToList}
            onOpenWorkflow={openWorkflow}
            agentOpen={agentOpen}
            onAgentOpenChange={setAgentOpen}
            listLabel={
              selected.kind === "apiEndpoint" || selected.apiProjectId
                ? "API Projects"
                : "Projects"
            }
          />
        ) : null}

        {!showEditor && section === "dashboard" ? (
          <DashboardPanel
            workflows={workflows}
            onOpenWorkflow={openWorkflow}
            onNavigate={(s) => navigate(s)}
            onToggleEnabled={(id, enabled) => void toggleWorkflowEnabled(id, enabled)}
          />
        ) : null}

        {!showEditor && section === "workflows" ? (
          <WorkflowList
            workflows={workflows.filter((w) => w.kind !== "apiEndpoint")}
            onOpen={openWorkflow}
            onCreate={() => void createWorkflow()}
            creating={creating}
            onToggleEnabled={(id, enabled) => void toggleWorkflowEnabled(id, enabled)}
          />
        ) : null}

        {!showEditor && section === "apis" ? (
          <ApiProjectsPanel
            onOpenWorkflow={openWorkflow}
            selectedProjectId={apiProjectId}
            onSelectedProjectIdChange={setApiProjectId}
          />
        ) : null}

        {section === "variables" ? <VariablesPanel /> : null}

        {section === "credentials" ? <CredentialsPanel /> : null}

        {section === "nodes" ? (
          <NodesPanel onAddNode={handleAddNode} canAdd={Boolean(selectedId && editorOpen)} />
        ) : null}

        {section === "runs" ? (
          <div className="h-full p-3">
            <div className="h-full overflow-hidden rounded-lg border">
              <RunsPanel refreshKey={refreshKey} />
            </div>
          </div>
        ) : null}

        {section === "insights" ? (
          <div className="h-full p-3">
            <div className="h-full overflow-hidden rounded-lg border">
              <InsightsPanel refreshKey={refreshKey} />
            </div>
          </div>
        ) : null}

        {section === "audit" ? (
          <div className="h-full p-3">
            <div className="h-full overflow-hidden rounded-lg border">
              <AuditPanel refreshKey={refreshKey} actorName={actorName} />
            </div>
          </div>
        ) : null}

        {inSettings ? (
          <SettingsPanel
            page={settingsPage}
            isAdmin={sessionUser.role === "admin"}
            onNavigate={setSettingsPage}
          />
        ) : null}
      </div>

      <Toaster theme={theme} />
    </div>
  );
}
