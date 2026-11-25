import { useCallback, useEffect, useMemo, useState } from "react";
import type { ApiProjectDoc, WorkflowDoc } from "@dataflow/shared";
import {
  Plus,
  ExternalLink,
  BookOpen,
  ChevronRight,
  ArrowLeft,
  Search,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import { httpMethodBadgeClass } from "@/lib/httpMethodStyles";

type ProjectDetail = {
  project: ApiProjectDoc;
  endpoints: Array<{
    _id: string;
    method: string;
    path: string;
    summary?: string;
    tag?: string;
    auth: string;
    workflowId: string;
  }>;
  workflows: WorkflowDoc[];
};

export function ApiProjectsPanel({
  onOpenWorkflow,
  selectedProjectId = null,
  onSelectedProjectIdChange,
}: {
  onOpenWorkflow: (id: string) => void;
  /** Preserved across editor open/close so Back returns to project detail. */
  selectedProjectId?: string | null;
  onSelectedProjectIdChange?: (id: string | null) => void;
}) {
  const [projects, setProjects] = useState<ApiProjectDoc[]>([]);
  const selectedId = selectedProjectId;
  const setSelectedId = onSelectedProjectIdChange ?? (() => undefined);
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newSlug, setNewSlug] = useState("");
  const [newVersion, setNewVersion] = useState("v1");
  const [addingEndpoint, setAddingEndpoint] = useState(false);
  const [epPath, setEpPath] = useState("");
  const [epMethod, setEpMethod] = useState("GET");
  const [editingVersion, setEditingVersion] = useState(false);
  const [versionDraft, setVersionDraft] = useState("");
  const [savingVersion, setSavingVersion] = useState(false);
  const [endpointQuery, setEndpointQuery] = useState("");

  const reload = useCallback(async () => {
    const res = await api<{ projects: ApiProjectDoc[] }>("/api/api-projects");
    setProjects(res.projects);
  }, []);

  const loadDetail = useCallback(async (id: string) => {
    const res = await api<ProjectDetail>(`/api/api-projects/${id}`);
    setDetail(res);
  }, []);

  useEffect(() => {
    void reload().catch((err) =>
      toast.error(err instanceof Error ? err.message : String(err)),
    );
  }, [reload]);

  useEffect(() => {
    setEditingVersion(false);
    setVersionDraft("");
    setEndpointQuery("");
    if (!selectedId) {
      setDetail(null);
      return;
    }
    void loadDetail(selectedId).catch((err) =>
      toast.error(err instanceof Error ? err.message : String(err)),
    );
  }, [selectedId, loadDetail]);

  const filteredEndpoints = useMemo(() => {
    if (!detail) return [];
    const q = endpointQuery.trim().toLowerCase();
    if (!q) return detail.endpoints;
    const version = (detail.project.version || "v1").toLowerCase();
    return detail.endpoints.filter((ep) => {
      const haystack = [
        ep.method,
        ep.path,
        ep.summary ?? "",
        ep.tag ?? "",
        ep.auth,
        version,
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });
  }, [detail, endpointQuery]);

  async function createProject() {
    setCreating(true);
    try {
      const res = await api<{ project: ApiProjectDoc }>("/api/api-projects", {
        method: "POST",
        body: JSON.stringify({
          name: newName.trim() || "New API",
          slug: newSlug.trim() || undefined,
          version: newVersion.trim() || "v1",
          auth: { mode: "jwt" },
        }),
      });
      setNewName("");
      setNewSlug("");
      setNewVersion("v1");
      await reload();
      setSelectedId(String(res.project._id));
      toast.success("API project created");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }

  async function addEndpoint() {
    if (!selectedId) return;
    setAddingEndpoint(true);
    try {
      const res = await api<{ workflow: WorkflowDoc }>(
        `/api/api-projects/${selectedId}/endpoints`,
        {
          method: "POST",
          body: JSON.stringify({
            path: epPath.trim() || undefined,
            method: epMethod,
          }),
        },
      );
      setEpPath("");
      await loadDetail(selectedId);
      toast.success("Endpoint flow created");
      onOpenWorkflow(String(res.workflow._id));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setAddingEndpoint(false);
    }
  }

  async function saveProjectVersion() {
    if (!selectedId || !detail) return;
    const next = versionDraft.trim() || "v1";
    setSavingVersion(true);
    try {
      await api(`/api/api-projects/${selectedId}`, {
        method: "PUT",
        body: JSON.stringify({ version: next }),
      });
      setEditingVersion(false);
      await loadDetail(selectedId);
      await reload();
      toast.success("Version updated");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setSavingVersion(false);
    }
  }

  async function toggleProjectEnabled(id: string, enabled: boolean) {
    try {
      await api(`/api/api-projects/${id}`, {
        method: "PUT",
        body: JSON.stringify({ enabled }),
      });
      setProjects((prev) =>
        prev.map((p) =>
          String(p._id) === id ? { ...p, enabled } : p,
        ),
      );
      if (selectedId === id && detail) {
        setDetail({
          ...detail,
          project: { ...detail.project, enabled },
        });
      }
      toast.success(enabled ? "Project enabled" : "Project disabled");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  }

  if (selectedId && detail) {
    const p = detail.project;
    const swaggerUrl = `/apis/${p.slug}/swagger`;
    const openapiUrl = `/apis/${p.slug}/openapi.json`;
    return (
      <div className="flex h-full flex-col">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5">
          <div>
            <button
              type="button"
              className="mb-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => setSelectedId(null)}
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              All API projects
            </button>
            <h1 className="text-2xl font-semibold tracking-tight">{p.name}</h1>
            <div className="mt-1 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
              <span>/apis/{p.slug}</span>
              <span>·</span>
              {editingVersion ? (
                <span className="inline-flex items-center gap-1.5">
                  <Input
                    className="h-7 w-20"
                    value={versionDraft}
                    onChange={(e) => setVersionDraft(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void saveProjectVersion();
                      if (e.key === "Escape") setEditingVersion(false);
                    }}
                    autoFocus
                  />
                  <Button
                    size="sm"
                    className="h-7 px-2"
                    disabled={savingVersion}
                    onClick={() => void saveProjectVersion()}
                  >
                    Save
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2"
                    disabled={savingVersion}
                    onClick={() => setEditingVersion(false)}
                  >
                    Cancel
                  </Button>
                </span>
              ) : (
                <button
                  type="button"
                  className="rounded px-1 font-medium text-foreground hover:bg-muted"
                  title="Edit version"
                  onClick={() => {
                    setVersionDraft(p.version || "v1");
                    setEditingVersion(true);
                  }}
                >
                  {p.version || "v1"}
                </button>
              )}
              <span>·</span>
              <span>
                {p.auth?.schemeName === "ApiKey"
                  ? "API Key"
                  : p.auth?.mode === "jwt"
                    ? "JWT"
                    : "Open"}{" "}
                auth
              </span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <a
              href={openapiUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-9 items-center gap-2 rounded-md border bg-background px-4 text-sm hover:bg-accent"
            >
              <BookOpen className="h-4 w-4" />
              OpenAPI
            </a>
            <a
              href={swaggerUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-9 items-center gap-2 rounded-md bg-primary px-4 text-sm text-primary-foreground hover:bg-primary/90"
            >
              <ExternalLink className="h-4 w-4" />
              Swagger UI
            </a>
          </div>
        </div>

        <div className="flex flex-wrap items-end gap-2 border-b px-6 py-3">
          <div className="min-w-[200px] flex-1">
            <Label className="text-xs text-muted-foreground">New path</Label>
            <Input
              className="mt-1 h-9"
              placeholder="Controller/Action"
              value={epPath}
              onChange={(e) => setEpPath(e.target.value)}
            />
          </div>
          <div>
            <Label className="text-xs text-muted-foreground">Method</Label>
            <FormSelect
              className="mt-1 w-[7.5rem]"
              value={epMethod}
              options={["GET", "POST", "PUT", "PATCH", "DELETE"].map((m) => ({
                value: m,
                label: m,
              }))}
              onChange={setEpMethod}
            />
          </div>
          <Button disabled={addingEndpoint} onClick={() => void addEndpoint()}>
            <Plus className="h-4 w-4" />
            Add endpoint
          </Button>
        </div>

        <ScrollArea className="min-h-0 flex-1">
          <div className="p-6">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <div className="relative min-w-[200px] flex-1 max-w-md">
                <Search className="pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground" />
                <Input
                  className="h-9 pl-8"
                  placeholder="Search endpoints…"
                  value={endpointQuery}
                  onChange={(e) => setEndpointQuery(e.target.value)}
                />
              </div>
              <div className="text-xs text-muted-foreground">
                {endpointQuery.trim()
                  ? `${filteredEndpoints.length} of ${detail.endpoints.length}`
                  : `${detail.endpoints.length} endpoint${detail.endpoints.length === 1 ? "" : "s"}`}
              </div>
            </div>
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="grid grid-cols-[90px_1.4fr_1fr_80px_100px_80px] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                <div>Method</div>
                <div>Path</div>
                <div>Summary</div>
                <div>Version</div>
                <div>Auth</div>
                <div />
              </div>
              {filteredEndpoints.map((ep) => (
                <button
                  key={ep._id}
                  type="button"
                  onClick={() => onOpenWorkflow(ep.workflowId)}
                  className="grid w-full grid-cols-[90px_1.4fr_1fr_80px_100px_80px] gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50"
                >
                  <div>
                    <Badge className={httpMethodBadgeClass(ep.method)}>
                      {ep.method}
                    </Badge>
                  </div>
                  <div className="truncate font-medium">{ep.path}</div>
                  <div className="truncate text-muted-foreground">
                    {ep.summary ?? ep.tag}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {p.version || "v1"}
                  </div>
                  <div className="text-xs text-muted-foreground">{ep.auth}</div>
                  <div className="flex justify-end text-muted-foreground">
                    <ChevronRight className="h-4 w-4" />
                  </div>
                </button>
              ))}
              {!detail.endpoints.length ? (
                <div className="px-4 py-8 text-center text-sm text-muted-foreground">
                  No endpoints yet — add one above.
                </div>
              ) : !filteredEndpoints.length ? (
                <div className="px-4 py-8 text-center text-sm text-muted-foreground">
                  No endpoints match “{endpointQuery.trim()}”.
                </div>
              ) : null}
            </div>
          </div>
        </ScrollArea>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">API Projects</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Multi-endpoint APIs with Swagger — each route is a flow
          </p>
        </div>
      </div>

      <div className="flex flex-wrap items-end gap-2 border-b px-6 py-3">
        <div className="min-w-[180px]">
          <Label className="text-xs text-muted-foreground">Name</Label>
          <Input
            className="mt-1 h-9"
            placeholder="My API"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
        </div>
        <div className="min-w-[140px]">
          <Label className="text-xs text-muted-foreground">Slug</Label>
          <Input
            className="mt-1 h-9"
            placeholder="my-api"
            value={newSlug}
            onChange={(e) => setNewSlug(e.target.value)}
          />
        </div>
        <div className="w-[100px]">
          <Label className="text-xs text-muted-foreground">Version</Label>
          <Input
            className="mt-1 h-9"
            placeholder="v1"
            value={newVersion}
            onChange={(e) => setNewVersion(e.target.value)}
          />
        </div>
        <Button disabled={creating} onClick={() => void createProject()}>
          <Plus className="h-4 w-4" />
          Create API
        </Button>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="p-6">
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="grid grid-cols-[minmax(0,1.5fr)_120px_80px_100px_140px_auto] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <div>Name</div>
              <div>Slug</div>
              <div>Version</div>
              <div>Status</div>
              <div>Updated</div>
              <div className="text-right">Swagger</div>
            </div>
            {projects.map((p) => (
              <div
                key={String(p._id)}
                role="button"
                tabIndex={0}
                onClick={() => setSelectedId(String(p._id))}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setSelectedId(String(p._id));
                  }
                }}
                className="grid w-full cursor-pointer grid-cols-[minmax(0,1.5fr)_120px_80px_100px_140px_auto] items-center gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50"
              >
                <div className="truncate font-medium">{p.name}</div>
                <div className="truncate text-muted-foreground">{p.slug}</div>
                <div className="truncate text-muted-foreground">
                  {p.version || "v1"}
                </div>
                <div
                  onClick={(e) => e.stopPropagation()}
                  onKeyDown={(e) => e.stopPropagation()}
                >
                  <Switch
                    size="sm"
                    checked={p.enabled}
                    onCheckedChange={(checked) =>
                      void toggleProjectEnabled(String(p._id), checked)
                    }
                    aria-label={p.enabled ? "Disable project" : "Enable project"}
                  />
                </div>
                <div className="truncate text-xs text-muted-foreground">
                  {new Date(p.updatedAt).toLocaleString()}
                </div>
                <a
                  href={`/apis/${p.slug}/swagger`}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-foreground"
                  onClick={(e) => e.stopPropagation()}
                  onKeyDown={(e) => e.stopPropagation()}
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                  Open Swagger
                </a>
              </div>
            ))}
            {!projects.length ? (
              <div className="px-4 py-8 text-center text-sm text-muted-foreground">
                No API projects yet. Create one, or start the server to seed
                Wealth.
              </div>
            ) : null}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
