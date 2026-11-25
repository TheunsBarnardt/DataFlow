import { useEffect, useMemo, useRef, useState } from "react";
import type {
  AuditEventDoc,
  RunDoc,
  WorkflowDoc,
  WorkflowNode,
} from "@dataflow/shared";
import {
  History,
  MoreHorizontal,
  FlaskConical,
  ChevronUp,
  ChevronDown,
  Boxes,
  MessageSquare,
  Copy,
  Download,
  Pencil,
  Share2,
  Trash2,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from "@/components/ui/drawer";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";
import {
  FlowCanvas,
  type PendingNodeAdd,
  type FlowCanvasHandle,
} from "@/components/FlowCanvas";
import { ChatPanel } from "@/components/ChatPanel";
import { NodeDetailsPanel } from "@/components/NodeDetailsPanel";
import { RunsPanel } from "@/components/RunsPanel";
import { NodesPanel, type NodeCatalogItem } from "@/components/NodesPanel";
import type { Theme } from "@/lib/theme";
import { api } from "@/lib/api";
import {
  httpMethodBadgeClass,
  splitHttpMethodName,
} from "@/lib/httpMethodStyles";
import { sampleFromApiEndpointParameters } from "@/lib/jsonSchemaSample";
import { toast } from "sonner";

type EditorTab = "editor" | "executions";

export function WorkflowEditor({
  workflow,
  theme,
  pendingAdd,
  refreshKey,
  onSaved,
  onAddNode,
  onBackToList,
  onOpenWorkflow,
  agentOpen: agentOpenProp,
  onAgentOpenChange,
  listLabel = "Projects",
}: {
  workflow: WorkflowDoc;
  theme: Theme;
  pendingAdd?: PendingNodeAdd | null;
  refreshKey: number;
  onSaved: () => void;
  onAddNode: (
    item: NodeCatalogItem,
    opts?: { connectFrom?: string; sourceHandle?: string },
  ) => void;
  onBackToList: () => void;
  onOpenWorkflow?: (id: string) => void;
  agentOpen?: boolean;
  onAgentOpenChange?: (open: boolean) => void;
  /** Breadcrumb parent: "Projects" or "API Projects". */
  listLabel?: string;
}) {
  const [tab, setTab] = useState<EditorTab>("editor");
  const [logsOpen, setLogsOpen] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [nodesOpen, setNodesOpen] = useState(false);
  const [triggersOnly, setTriggersOnly] = useState(false);
  const [connectFrom, setConnectFrom] = useState<string | undefined>();
  const [agentOpenLocal, setAgentOpenLocal] = useState(false);
  const agentOpen = agentOpenProp ?? agentOpenLocal;
  const setAgentOpen = onAgentOpenChange ?? setAgentOpenLocal;
  const [selectedNode, setSelectedNode] = useState<WorkflowNode | null>(null);
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [overlayRun, setOverlayRun] = useState<RunDoc | null>(null);
  const [nodeSamples, setNodeSamples] = useState<Record<string, unknown>>({});
  const [renaming, setRenaming] = useState(false);
  const [nameDraft, setNameDraft] = useState(workflow.name);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historyEvents, setHistoryEvents] = useState<AuditEventDoc[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const canvasRef = useRef<FlowCanvasHandle>(null);

  useEffect(() => {
    setNameDraft(workflow.name);
  }, [workflow._id, workflow.name]);

  useEffect(() => {
    setOverlayRun(null);
    void api<{ runs: RunDoc[] }>(
      `/api/runs?workflowId=${encodeURIComponent(String(workflow._id))}&limit=1`,
    )
      .then(async (r) => {
        const id = r.runs[0]?._id;
        if (!id) return;
        const detail = await api<{ run: RunDoc }>(`/api/runs/${id}`);
        setOverlayRun(detail.run);
      })
      .catch(() => {
        /* ignore — overlay is optional */
      });
  }, [workflow._id, refreshKey]);

  useEffect(() => {
    if (renaming) {
      nameInputRef.current?.focus();
      nameInputRef.current?.select();
    }
  }, [renaming]);

  useEffect(() => {
    if (!historyOpen) return;
    setHistoryLoading(true);
    void api<{ events: AuditEventDoc[] }>(
      `/api/audit?resourceId=${encodeURIComponent(String(workflow._id))}&limit=50`,
    )
      .then((r) => setHistoryEvents(r.events))
      .catch((err) => {
        toast.error(err instanceof Error ? err.message : "Failed to load history");
        setHistoryEvents([]);
      })
      .finally(() => setHistoryLoading(false));
  }, [historyOpen, workflow._id, refreshKey]);

  async function save() {
    setSaving(true);
    try {
      await canvasRef.current?.save();
    } finally {
      setSaving(false);
    }
  }

  async function run() {
    setRunning(true);
    try {
      await canvasRef.current?.run();
      setLogsOpen(true);
      const list = await api<{ runs: RunDoc[] }>(
        `/api/runs?workflowId=${encodeURIComponent(String(workflow._id))}&limit=1`,
      );
      const id = list.runs[0]?._id;
      if (id) {
        const detail = await api<{ run: RunDoc }>(`/api/runs/${id}`);
        setOverlayRun(detail.run);
      }
    } finally {
      setRunning(false);
    }
  }

  async function toggleEnabled() {
    await canvasRef.current?.save();
    await api(`/api/workflows/${workflow._id}`, {
      method: "PUT",
      body: JSON.stringify({ enabled: !workflow.enabled }),
    });
    onSaved();
  }

  async function commitRename() {
    const name = nameDraft.trim();
    setRenaming(false);
    if (!name || name === workflow.name) {
      setNameDraft(workflow.name);
      return;
    }
    try {
      await api(`/api/workflows/${workflow._id}`, {
        method: "PUT",
        body: JSON.stringify({ name }),
      });
      toast.success("Workflow renamed");
      onSaved();
    } catch (err) {
      setNameDraft(workflow.name);
      toast.error(err instanceof Error ? err.message : "Rename failed");
    }
  }

  async function duplicateWorkflow() {
    try {
      await canvasRef.current?.save();
      const { workflow: latest } = await api<{ workflow: WorkflowDoc }>(
        `/api/workflows/${workflow._id}`,
      );
      const res = await api<{ workflow: WorkflowDoc }>("/api/workflows", {
        method: "POST",
        body: JSON.stringify({
          name: `${latest.name} copy`,
          description: latest.description,
          enabled: false,
          nodes: latest.nodes,
          edges: latest.edges,
          tags: latest.tags ?? [],
          timeSavedMinutes: latest.timeSavedMinutes,
        }),
      });
      toast.success("Workflow duplicated");
      onSaved();
      onOpenWorkflow?.(String(res.workflow._id));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Duplicate failed");
    }
  }

  function downloadWorkflow() {
    const payload = {
      name: workflow.name,
      description: workflow.description,
      enabled: workflow.enabled,
      version: workflow.version,
      tags: workflow.tags ?? [],
      timeSavedMinutes: workflow.timeSavedMinutes,
      nodes: workflow.nodes,
      edges: workflow.edges,
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${workflow.name.replace(/[^\w.-]+/g, "_") || "workflow"}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success("Workflow downloaded");
  }

  async function deleteWorkflow() {
    if (
      !window.confirm(
        `Delete “${workflow.name}”? This cannot be undone.`,
      )
    ) {
      return;
    }
    try {
      await api(`/api/workflows/${workflow._id}`, { method: "DELETE" });
      toast.success("Workflow deleted");
      onBackToList();
      onSaved();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Delete failed");
    }
  }

  const [connectFromHandle, setConnectFromHandle] = useState<
    string | undefined
  >();

  function openNodesPicker(opts?: {
    connectFrom?: string;
    sourceHandle?: string;
    triggersOnly?: boolean;
  }) {
    setConnectFrom(opts?.connectFrom);
    setConnectFromHandle(opts?.sourceHandle);
    setTriggersOnly(Boolean(opts?.triggersOnly));
    setAgentOpen(false);
    setDetailsOpen(false);
    setSelectedNode(null);
    setNodesOpen(true);
  }

  function logForNode(nodeId: string) {
    return overlayRun?.logs?.find((l) => l.nodeId === nodeId);
  }

  const detailsInputSample = useMemo(() => {
    if (!selectedNode) return undefined;
    const log = logForNode(selectedNode.id);
    if (log?.input != null) return log.input;
    const upstream = workflow.edges.find((e) => e.target === selectedNode.id);
    if (!upstream) {
      // Trigger: surface request schema as Input so query/body params are visible for mapping
      if (selectedNode.type === "apiEndpoint") {
        return sampleFromApiEndpointParameters(selectedNode.parameters);
      }
      return undefined;
    }
    const upLog = logForNode(upstream.source);
    if (upLog?.output != null) return upLog.output;
    if (nodeSamples[upstream.source] != null) {
      return nodeSamples[upstream.source];
    }
    const upNode = workflow.nodes.find((n) => n.id === upstream.source);
    if (upNode?.type === "apiEndpoint") {
      return sampleFromApiEndpointParameters(upNode.parameters);
    }
    return undefined;
  }, [selectedNode, workflow.edges, workflow.nodes, nodeSamples, overlayRun]);

  const detailsOutputSample = useMemo(() => {
    if (!selectedNode) return undefined;
    const log = logForNode(selectedNode.id);
    if (log?.output != null) return log.output;
    if (nodeSamples[selectedNode.id] != null) {
      return nodeSamples[selectedNode.id];
    }
    if (selectedNode.type === "apiEndpoint") {
      return sampleFromApiEndpointParameters(selectedNode.parameters);
    }
    return undefined;
  }, [selectedNode, nodeSamples, overlayRun]);

  function selectNodeForDetails(n: WorkflowNode | null) {
    setNodesOpen(false);
    setAgentOpen(false);
    setSelectedNode(n);
    setDetailsOpen(Boolean(n));
    if (n) {
      const log = overlayRun?.logs?.find((l) => l.nodeId === n.id);
      if (log?.output != null) {
        setNodeSamples((prev) => ({ ...prev, [n.id]: log.output }));
      }
    }
  }

  async function executeNodeStep(nodeId: string) {
    try {
      const upstream = workflow.edges.find((e) => e.target === nodeId);
      const input =
        nodeSamples[nodeId] ??
        (upstream ? nodeSamples[upstream.source] : undefined);
      const meta = canvasRef.current?.getNode(nodeId);
      const res = await api<{
        output: unknown;
        error?: string;
      }>(`/api/workflows/${workflow._id}/execute-node`, {
        method: "POST",
        body: JSON.stringify({
          nodeId,
          input,
          parameters: meta?.parameters,
        }),
      });
      if (res.error) {
        toast.error(res.error);
        setNodeSamples((prev) => ({ ...prev, [nodeId]: { error: res.error } }));
      } else {
        setNodeSamples((prev) => ({ ...prev, [nodeId]: res.output }));
        toast.success("Step executed");
        const meta = canvasRef.current?.getNode(nodeId) ?? null;
        if (meta) {
          setSelectedNode(meta);
          setDetailsOpen(true);
        }
      }
      onSaved();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Execute failed");
    }
  }

  // Tab → open nodes panel; Esc → close creator; Ctrl/Cmd+Enter → execute step
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const el = e.target as HTMLElement | null;
      const tag = el?.tagName?.toLowerCase();
      const typing =
        tag === "input" ||
        tag === "textarea" ||
        tag === "select" ||
        Boolean(el?.isContentEditable);

      if (e.key === "Escape" && nodesOpen) {
        e.preventDefault();
        setNodesOpen(false);
        setTriggersOnly(false);
        setConnectFrom(undefined);
        setConnectFromHandle(undefined);
        return;
      }

      if (
        e.key === "Tab" &&
        !typing &&
        !e.ctrlKey &&
        !e.metaKey &&
        !e.altKey &&
        tab === "editor"
      ) {
        e.preventDefault();
        openNodesPicker();
        return;
      }

      if (
        (e.ctrlKey || e.metaKey) &&
        e.key === "Enter" &&
        detailsOpen &&
        selectedNode &&
        tab === "editor"
      ) {
        e.preventDefault();
        void executeNodeStep(selectedNode.id);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    tab,
    detailsOpen,
    selectedNode,
    nodesOpen,
    nodeSamples,
    detailsInputSample,
    workflow._id,
  ]);

  const canvas = (
    <FlowCanvas
      ref={canvasRef}
      workflow={workflow}
      onSaved={onSaved}
      pendingAdd={tab === "editor" ? pendingAdd : null}
      colorMode={theme}
      hideChrome
      readOnly={tab === "executions"}
      runLogs={overlayRun?.logs ?? null}
      onRequestAddNode={
        tab === "editor" ? openNodesPicker : undefined
      }
      onExecuteNode={
        tab === "editor" ? (id) => void executeNodeStep(id) : undefined
      }
      onNodeSelect={selectNodeForDetails}
    />
  );

  const mainContent =
    tab === "editor" ? (
      <>
        <div className="absolute right-3 top-3 z-10 flex items-center gap-2">
          <Drawer
            direction="right"
            open={agentOpen}
            onOpenChange={(open) => {
              setAgentOpen(open);
              if (open) {
                setNodesOpen(false);
                setDetailsOpen(false);
                setSelectedNode(null);
              }
            }}
            modal={false}
          >
            <DrawerTrigger asChild>
              <Button
                size="sm"
                variant="outline"
                className="h-8 border-border/80 bg-card/95 shadow-md backdrop-blur-sm"
              >
                <MessageSquare className="h-4 w-4" />
                Agent
              </Button>
            </DrawerTrigger>
            <DrawerContent className="h-full w-[min(100vw,36rem)] sm:max-w-[36rem]">
              <DrawerHeader className="sr-only">
                <DrawerTitle>Agent</DrawerTitle>
                <DrawerDescription>
                  Chat with the agent to build and edit workflows.
                </DrawerDescription>
              </DrawerHeader>
              <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                <ChatPanel compact onWorkflowChanged={onSaved} />
              </div>
            </DrawerContent>
          </Drawer>

          <Button
            size="sm"
            variant="outline"
            className="h-8 border-border/80 bg-card/95 shadow-md backdrop-blur-sm"
            onClick={() => openNodesPicker()}
          >
            <Boxes className="h-4 w-4" />
            Nodes
          </Button>
        </div>

        {nodesOpen ? (
          <div
            className="absolute inset-0 z-50 flex items-start justify-center bg-black/40 p-4 pt-[8vh] backdrop-blur-[1px]"
            onClick={() => {
              setNodesOpen(false);
              setTriggersOnly(false);
              setConnectFrom(undefined);
              setConnectFromHandle(undefined);
            }}
          >
            <div
              className="flex max-h-[80vh] w-full max-w-3xl flex-col overflow-hidden rounded-xl border bg-card shadow-2xl"
              onClick={(e) => e.stopPropagation()}
              role="dialog"
              aria-modal="true"
              aria-label={triggersOnly ? "Choose a trigger" : "Add node"}
            >
              <div className="flex items-start justify-between gap-3 border-b px-4 py-3">
                <div>
                  <h2 className="text-base font-semibold">
                    {triggersOnly ? "Choose a trigger" : "What happens next?"}
                  </h2>
                  <p className="text-xs text-muted-foreground">
                    {triggersOnly
                      ? "Pick how this workflow should start."
                      : "Search or pick a node. Tab opens this panel · Esc closes."}
                  </p>
                </div>
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label="Close"
                  onClick={() => {
                    setNodesOpen(false);
                    setTriggersOnly(false);
                    setConnectFrom(undefined);
                    setConnectFromHandle(undefined);
                  }}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
              <div className="min-h-0 flex-1 overflow-hidden">
                <NodesPanel
                  compact
                  hideHeader
                  canAdd
                  variant="grid"
                  triggersOnly={triggersOnly}
                  onAddNode={(item) => {
                    onAddNode(item, {
                      connectFrom,
                      sourceHandle: connectFromHandle,
                    });
                    setConnectFrom(undefined);
                    setConnectFromHandle(undefined);
                    setTriggersOnly(false);
                    setNodesOpen(false);
                  }}
                />
              </div>
            </div>
          </div>
        ) : null}

        <div className="h-full">{canvas}</div>

        <div className="pointer-events-none absolute inset-x-0 bottom-4 z-10 flex justify-center">
          <Button
            size="lg"
            className="pointer-events-auto h-11 rounded-xl px-6 text-[13px] font-semibold tracking-tight shadow-[0_8px_28px_color-mix(in_oklch,var(--brand)_35%,transparent)]"
            disabled={running}
            onClick={() => void run()}
          >
            <FlaskConical className="h-4 w-4" />
            {running ? "Executing…" : "Execute workflow"}
          </Button>
        </div>
      </>
    ) : (
      <div className="flex h-full min-h-0">
        <div className="w-[min(100%,20rem)] shrink-0 border-r bg-card">
          <RunsPanel
            refreshKey={refreshKey}
            workflowId={String(workflow._id)}
            listOnly
            selectedRunId={overlayRun?._id ? String(overlayRun._id) : null}
            onSelectRun={setOverlayRun}
          />
        </div>
        <div className="relative min-h-0 min-w-0 flex-1">
          {canvas}
          {overlayRun ? (
            <div className="pointer-events-none absolute left-3 top-3 z-10 rounded-md border bg-card/95 px-2.5 py-1.5 text-xs shadow-sm backdrop-blur">
              <span className="font-medium">
                {new Date(overlayRun.startedAt).toLocaleString()}
              </span>
              <span className="text-muted-foreground">
                {" "}
                · {overlayRun.status}
                {overlayRun._id ? ` · ID#${overlayRun._id.slice(-4)}` : ""}
              </span>
            </div>
          ) : null}
        </div>
      </div>
    );

  const logsTrigger = (
    <CollapsibleTrigger className="flex w-full shrink-0 items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground hover:bg-muted/50">
      {logsOpen ? (
        <ChevronDown className="h-3.5 w-3.5" />
      ) : (
        <ChevronUp className="h-3.5 w-3.5" />
      )}
      Logs
    </CollapsibleTrigger>
  );

  const { method: nameMethod, rest: nameRest } = splitHttpMethodName(
    workflow.name,
  );

  return (
    <div className="relative flex h-full min-w-0 flex-1 flex-col bg-background">
      {/* n8n-style main header */}
      <header className="relative flex h-12 shrink-0 items-center gap-3 border-b border-border/70 bg-card/40 px-3 backdrop-blur-md">
        <div className="flex min-w-0 flex-1 items-center gap-1.5 text-sm">
          <button
            type="button"
            onClick={onBackToList}
            className="rounded-md px-1 py-0.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
          >
            {listLabel}
          </button>
          <span className="text-muted-foreground/70">/</span>
          {renaming ? (
            <Input
              ref={nameInputRef}
              value={nameDraft}
              onChange={(e) => setNameDraft(e.target.value)}
              onBlur={() => void commitRename()}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void commitRename();
                }
                if (e.key === "Escape") {
                  setNameDraft(workflow.name);
                  setRenaming(false);
                }
              }}
              className="h-8 max-w-[18rem] px-1.5 py-0 text-base font-semibold"
            />
          ) : (
            <button
              type="button"
              title="Rename"
              onClick={() => setRenaming(true)}
              className="flex min-w-0 items-center gap-1.5 truncate rounded-md px-1 py-0.5 text-base font-semibold tracking-tight hover:bg-muted/50"
            >
              {nameMethod ? (
                <>
                  <Badge
                    className={cn(
                      "shrink-0 font-semibold no-underline",
                      httpMethodBadgeClass(nameMethod),
                    )}
                  >
                    {nameMethod}
                  </Badge>
                  <span className="truncate">{nameRest}</span>
                </>
              ) : (
                workflow.name
              )}
            </button>
          )}
        </div>

        <div className="ml-auto flex flex-1 items-center justify-end gap-2">
          <Button
            size="sm"
            className="h-8 shadow-sm"
            disabled={saving}
            onClick={() => void save()}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
          <Drawer
            direction="right"
            open={historyOpen}
            onOpenChange={setHistoryOpen}
          >
            <DrawerContent className="w-[min(100vw,22rem)] sm:max-w-[22rem]">
              <DrawerHeader>
                <DrawerTitle>History</DrawerTitle>
                <DrawerDescription>
                  Audit trail for this workflow (v{workflow.version}).
                </DrawerDescription>
              </DrawerHeader>
              <ScrollArea className="min-h-0 flex-1 px-4 pb-4">
                <div className="flex flex-col gap-2">
                  {historyEvents.map((e) => (
                    <div
                      key={e._id}
                      className="rounded-md border bg-card p-2 text-sm"
                    >
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge variant="outline">{e.action}</Badge>
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        <span className="font-medium text-foreground">
                          {e.actor}
                        </span>
                        {" · "}
                        {new Date(e.at).toLocaleString()}
                        {e.details && Object.keys(e.details).length ? (
                          <>
                            {" · "}
                            <span className="font-mono text-[10px]">
                              {JSON.stringify(e.details)}
                            </span>
                          </>
                        ) : null}
                      </div>
                    </div>
                  ))}
                  {!historyEvents.length && !historyLoading ? (
                    <div className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
                      No history yet. Save or run this workflow to populate it.
                    </div>
                  ) : null}
                  {historyLoading ? (
                    <div className="text-sm text-muted-foreground">
                      Loading history…
                    </div>
                  ) : null}
                </div>
              </ScrollArea>
            </DrawerContent>
          </Drawer>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
                title="More"
              >
                <MoreHorizontal className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              <DropdownMenuLabel>Workflow</DropdownMenuLabel>
              <DropdownMenuItem
                onSelect={(e) => {
                  e.preventDefault();
                  void toggleEnabled();
                }}
                className="justify-between gap-4"
              >
                <span>{workflow.enabled ? "Active" : "Inactive"}</span>
                <Switch
                  size="sm"
                  checked={workflow.enabled}
                  tabIndex={-1}
                  className="pointer-events-none"
                  aria-hidden
                />
              </DropdownMenuItem>
              <DropdownMenuItem>
                <Share2 />
                Share
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setHistoryOpen(true)}>
                <History />
                History
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => setRenaming(true)}>
                <Pencil />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void duplicateWorkflow()}>
                <Copy />
                Duplicate
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={downloadWorkflow}>
                <Download />
                Download
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onSelect={() => void deleteWorkflow()}
              >
                <Trash2 />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>

      {/* Outside header: backdrop-blur clips overflow; straddle the border from the page shell */}
      <div className="pointer-events-none absolute left-1/2 top-12 z-[100] flex h-8 -translate-x-1/2 -translate-y-1/2 items-stretch rounded-md bg-muted p-0.5 shadow-sm">
        {(
          [
            ["editor", "Editor"],
            ["executions", "Executions"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={cn(
              "pointer-events-auto inline-flex items-center justify-center rounded-sm px-3.5 text-xs font-medium leading-none transition-colors",
              tab === id
                ? "bg-background text-foreground"
                : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
            )}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="relative flex min-h-0 flex-1">
        <Collapsible
          open={logsOpen}
          onOpenChange={setLogsOpen}
          className="flex min-h-0 min-w-0 flex-1 flex-col"
        >
          {logsOpen ? (
            <ResizablePanelGroup
              orientation="vertical"
              className="min-h-0 flex-1"
            >
              <ResizablePanel
                defaultSize="70%"
                minSize="40%"
                className="relative min-h-0"
              >
                {mainContent}
              </ResizablePanel>
              <ResizableHandle withHandle />
              <ResizablePanel
                defaultSize="30%"
                minSize="15%"
                maxSize="60%"
                className="min-h-0"
              >
                <div className="flex h-full min-h-0 flex-col border-t bg-card">
                  {logsTrigger}
                  <CollapsibleContent className="min-h-0 flex-1 overflow-hidden data-[state=open]:flex data-[state=open]:min-h-0 data-[state=open]:flex-1 data-[state=open]:flex-col">
                    <RunsPanel
                      refreshKey={refreshKey}
                      workflowId={String(workflow._id)}
                    />
                  </CollapsibleContent>
                </div>
              </ResizablePanel>
            </ResizablePanelGroup>
          ) : (
            <>
              <div className="relative min-h-0 flex-1">{mainContent}</div>
              <div className="shrink-0 border-t bg-card">{logsTrigger}</div>
            </>
          )}
        </Collapsible>

        {detailsOpen && selectedNode ? (
          <div className="absolute inset-0 z-40 min-w-0 flex-1">
            <NodeDetailsPanel
              selectedNode={selectedNode}
              workflowId={String(workflow._id)}
              workflowEnabled={workflow.enabled}
              sampleOutput={detailsOutputSample}
              inputSample={detailsInputSample}
              readOnly={tab === "executions"}
              onSampleOutput={(nodeId, sample) =>
                setNodeSamples((prev) => ({ ...prev, [nodeId]: sample }))
              }
              onRunRecorded={onSaved}
              onClose={() => {
                setDetailsOpen(false);
                setSelectedNode(null);
              }}
              onUpdateNode={
                tab === "executions"
                  ? undefined
                  : async (id, patch) => {
                      const next =
                        canvasRef.current?.updateNode(id, patch) ?? null;
                      if (!next) {
                        throw new Error("Failed to update node on canvas");
                      }
                      setSelectedNode(next);
                      await canvasRef.current?.save({ silent: true });
                    }
              }
              onDeleteNode={(id) => {
                canvasRef.current?.deleteNode(id);
                setDetailsOpen(false);
                setSelectedNode(null);
              }}
            />
          </div>
        ) : null}
      </div>
    </div>
  );
}
