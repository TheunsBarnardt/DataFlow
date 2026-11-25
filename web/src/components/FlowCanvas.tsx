import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type MouseEvent as ReactMouseEvent,
} from "react";
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  addEdge,
  useEdgesState,
  useNodesState,
  MarkerType,
  type Connection,
  type Edge,
  type EdgeProps,
  type NodeTypes,
  type EdgeTypes,
  type OnNodesDelete,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Copy,
  CopyPlus,
  Eraser,
  PanelRightOpen,
  Play,
  Plus,
  Power,
  PowerOff,
  SquareMousePointer,
  TextCursorInput,
  Trash2,
} from "lucide-react";
import type {
  NodeRunLog,
  NodeType,
  WorkflowDoc,
  WorkflowNode,
} from "@dataflow/shared";
import { api } from "@/lib/api";
import { toast } from "sonner";
import {
  WorkflowCanvasNode,
  type WorkflowCanvasFlowNode,
  type WorkflowCanvasNodeData,
} from "@/components/WorkflowCanvasNode";
import { defaultParameters } from "@/lib/nodeSchemas";
import { getNodeConfigIssues } from "@/lib/nodeConfigIssues";
import { connectionLabelForNode } from "@/lib/nodeHandles";
import { shortId } from "@/lib/id";
import {
  buildNodeOverlayMap,
  itemCountFromOutput,
} from "@/lib/runOverlay";
import { Button } from "@/components/ui/button";

export interface PendingNodeAdd {
  type: NodeType;
  label: string;
  token: number;
  /** When set, connect new node from this source */
  connectFrom?: string;
  /** Source handle id when connecting from a specific output */
  sourceHandle?: string;
}

export type NodeUpdatePatch = Partial<
  Pick<WorkflowNode, "name" | "parameters" | "type" | "settings">
>;

export type FlowCanvasSaveOptions = {
  /** Skip the “Workflow saved” toast (e.g. when Apply already notifies). */
  silent?: boolean;
};

export interface FlowCanvasHandle {
  save: (opts?: FlowCanvasSaveOptions) => Promise<void>;
  run: () => Promise<void>;
  updateNode: (id: string, patch: NodeUpdatePatch) => WorkflowNode | null;
  deleteNode: (id: string) => void;
  getNode: (id: string) => WorkflowNode | null;
}

const nodeTypes: NodeTypes = {
  workflow: WorkflowCanvasNode,
};

type EdgeOverlayData = {
  onAdd?: (edgeId: string) => void;
  onDelete?: (edgeId: string) => void;
  itemCount?: number;
  executed?: boolean;
  failed?: boolean;
  /** Idle label (e.g. webhook POST) when no run item-count pill. */
  connectionLabel?: string;
};

function toFlowNode(
  n: WorkflowNode,
  handlers: {
    onEdit?: (id: string) => void;
    onExecute?: (id: string) => void;
    onToggleDisabled?: (id: string) => void;
    onAddFromHandle?: (id: string, sourceHandle: string) => void;
  },
  overlay?: Pick<WorkflowCanvasNodeData, "runStatus" | "durationMs">,
): WorkflowCanvasFlowNode {
  const disabled = Boolean(n.settings?.disabled);
  const issues = getNodeConfigIssues(n.type, n.parameters, { disabled });
  const httpMethod =
    n.type === "webhook" || n.type === "apiEndpoint"
      ? connectionLabelForNode(n.type, n.parameters)
      : undefined;
  return {
    id: n.id,
    type: "workflow",
    position: n.position,
    data: {
      name: n.name,
      type: n.type,
      disabled,
      note:
        n.type === "stickyNote"
          ? String(n.parameters?.content ?? n.settings?.notes ?? "")
          : n.settings?.displayNoteInFlow && n.settings.notes
            ? n.settings.notes
            : undefined,
      color:
        n.type === "stickyNote" && typeof n.parameters?.color === "string"
          ? n.parameters.color
          : undefined,
      switchCases: switchHandleIds(n),
      errorOutput: n.settings?.onError === "continueErrorOutput",
      runStatus: overlay?.runStatus,
      durationMs: overlay?.durationMs,
      httpMethod,
      hasIssues: issues.length > 0,
      issues: issues.length ? issues : undefined,
      onEdit: handlers.onEdit,
      onExecute: handlers.onExecute,
      onToggleDisabled: handlers.onToggleDisabled,
      onAddFromHandle: handlers.onAddFromHandle,
    },
  };
}

function switchHandleIds(n: WorkflowNode): string[] | undefined {
  if (n.type !== "switch") return undefined;
  const mode = String(n.parameters?.mode ?? "rules");
  if (mode === "rules" && Array.isArray(n.parameters?.rules)) {
    const keys = (n.parameters.rules as Array<{ outputKey?: string }>)
      .map((r, i) => String(r.outputKey ?? `output${i}`))
      .filter(Boolean);
    const fallback = String(n.parameters?.fallbackOutput ?? "default");
    return [...new Set([...keys, fallback])];
  }
  if (Array.isArray(n.parameters?.cases)) {
    return n.parameters.cases as string[];
  }
  return undefined;
}

function AddEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style,
  markerEnd,
  data,
}: EdgeProps) {
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });
  const edgeData = data as EdgeOverlayData | undefined;
  const showItems = edgeData?.executed && edgeData.itemCount != null;
  const itemLabel =
    edgeData?.itemCount === 1
      ? "1 item"
      : `${edgeData?.itemCount ?? 0} items`;
  const showConnectionLabel =
    !showItems && Boolean(edgeData?.connectionLabel);
  return (
    <>
      <BaseEdge
        path={edgePath}
        markerEnd={markerEnd}
        style={style}
        interactionWidth={24}
      />
      <EdgeLabelRenderer>
        <div
          className="nodrag nopan pointer-events-auto absolute flex -translate-x-1/2 -translate-y-1/2 items-center gap-1"
          style={{ left: labelX, top: labelY }}
        >
          {showItems ? (
            <span
              className={
                edgeData?.failed
                  ? "rounded-full border border-destructive/40 bg-destructive/10 px-1.5 py-0.5 text-[10px] font-medium text-destructive shadow-sm"
                  : "rounded-full border border-emerald-500/40 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-300 shadow-sm"
              }
              title={edgeData?.failed ? "Source step failed" : "Items passed"}
            >
              {itemLabel}
            </span>
          ) : null}
          {showConnectionLabel ? (
            <span className="rounded bg-[var(--canvas)] px-1 py-0.5 text-[10px] font-medium text-muted-foreground">
              {edgeData?.connectionLabel}
            </span>
          ) : null}
          {edgeData?.onAdd ? (
            <button
              type="button"
              className="flex h-5 w-5 items-center justify-center rounded-full border bg-card text-muted-foreground shadow-sm hover:border-[var(--brand)] hover:text-foreground"
              title="Add node"
              onClick={(e) => {
                e.stopPropagation();
                edgeData.onAdd?.(id);
              }}
            >
              <Plus className="h-3 w-3" />
            </button>
          ) : null}
          {edgeData?.onDelete ? (
            <button
              type="button"
              className="flex h-5 w-5 items-center justify-center rounded-full border bg-card text-muted-foreground shadow-sm hover:border-destructive hover:bg-destructive/10 hover:text-destructive"
              title="Delete connection"
              onClick={(e) => {
                e.stopPropagation();
                edgeData.onDelete?.(id);
              }}
            >
              <Trash2 className="h-3 w-3" />
            </button>
          ) : null}
        </div>
      </EdgeLabelRenderer>
    </>
  );
}

const edgeTypes: EdgeTypes = {
  addable: AddEdge,
};

function MenuItem({
  icon: Icon,
  label,
  shortcut,
  destructive,
  onClick,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  shortcut?: string;
  destructive?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className={
        destructive
          ? "flex w-full cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-destructive outline-hidden select-none hover:bg-destructive/10"
          : "flex w-full cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none hover:bg-accent hover:text-accent-foreground"
      }
      onClick={onClick}
    >
      <Icon className="size-4 shrink-0 opacity-80" />
      <span className="flex-1 text-left">{label}</span>
      {shortcut ? (
        <span className="ml-4 text-[11px] text-muted-foreground tabular-nums">
          {shortcut}
        </span>
      ) : null}
    </button>
  );
}

function MenuSep() {
  return <div role="separator" className="-mx-1 my-1 h-px bg-border" />;
}

function clampMenuPos(x: number, y: number, width = 240, height = 360) {
  const pad = 8;
  const maxX = Math.max(pad, window.innerWidth - width - pad);
  const maxY = Math.max(pad, window.innerHeight - height - pad);
  return {
    left: Math.min(Math.max(pad, x), maxX),
    top: Math.min(Math.max(pad, y), maxY),
  };
}

function NodeContextMenu({
  x,
  y,
  disabled,
  onOpen,
  onExecute,
  onRename,
  onToggleDisabled,
  onCopy,
  onDuplicate,
  onSelectAll,
  onClearSelection,
  onDelete,
}: {
  x: number;
  y: number;
  disabled: boolean;
  onOpen: () => void;
  onExecute: () => void;
  onRename: () => void;
  onToggleDisabled: () => void;
  onCopy: () => void;
  onDuplicate: () => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
  onDelete: () => void;
}) {
  const pos = clampMenuPos(x, y);
  return (
    <div
      role="menu"
      className="fixed z-50 min-w-56 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
      style={pos}
    >
      <MenuItem
        icon={PanelRightOpen}
        label="Open..."
        shortcut="↵"
        onClick={onOpen}
      />
      <MenuItem icon={Play} label="Execute step" onClick={onExecute} />
      <MenuSep />
      <MenuItem
        icon={TextCursorInput}
        label="Rename"
        shortcut="Space"
        onClick={onRename}
      />
      <MenuItem
        icon={disabled ? Power : PowerOff}
        label={disabled ? "Activate" : "Deactivate"}
        shortcut="D"
        onClick={onToggleDisabled}
      />
      <MenuSep />
      <MenuItem icon={Copy} label="Copy" shortcut="Ctrl+C" onClick={onCopy} />
      <MenuItem
        icon={CopyPlus}
        label="Duplicate"
        shortcut="Ctrl+D"
        onClick={onDuplicate}
      />
      <MenuSep />
      <MenuItem
        icon={SquareMousePointer}
        label="Select all"
        shortcut="Ctrl+A"
        onClick={onSelectAll}
      />
      <MenuItem
        icon={Eraser}
        label="Clear selection"
        onClick={onClearSelection}
      />
      <MenuSep />
      <MenuItem
        icon={Trash2}
        label="Delete"
        shortcut="Del"
        destructive
        onClick={onDelete}
      />
    </div>
  );
}

export const FlowCanvas = forwardRef<
  FlowCanvasHandle,
  {
    workflow: WorkflowDoc | null;
    onSaved: () => void;
    pendingAdd?: PendingNodeAdd | null;
    colorMode?: "dark" | "light";
    hideChrome?: boolean;
    /** Node logs from the selected / latest run — drives status badges & edge item counts. */
    runLogs?: NodeRunLog[] | null;
    /** When true, hide add-node chrome (executions preview). */
    readOnly?: boolean;
    onNodeSelect?: (node: WorkflowNode | null) => void;
    onRequestAddNode?: (opts?: {
      connectFrom?: string;
      sourceHandle?: string;
      triggersOnly?: boolean;
    }) => void;
    onExecuteNode?: (nodeId: string) => void;
  }
>(function FlowCanvas(
  {
    workflow,
    onSaved,
    pendingAdd,
    colorMode = "dark",
    hideChrome = false,
    runLogs = null,
    readOnly = false,
    onNodeSelect,
    onRequestAddNode,
    onExecuteNode,
  },
  ref,
) {
  const metaRef = useRef<Map<string, WorkflowNode>>(new Map());
  // Keep a render-synced lookup so makeEdge can resolve labels before the sync effect runs.
  const nodesByIdRef = useRef<Map<string, WorkflowNode>>(new Map());
  {
    const map = new Map<string, WorkflowNode>();
    for (const n of workflow?.nodes ?? []) map.set(n.id, n);
    // Merge unsaved local meta over server nodes
    for (const [id, n] of metaRef.current) map.set(id, n);
    nodesByIdRef.current = map;
  }
  const lastAddToken = useRef<number>(0);
  const onNodeSelectRef = useRef(onNodeSelect);
  onNodeSelectRef.current = onNodeSelect;
  const onRequestAddNodeRef = useRef(onRequestAddNode);
  onRequestAddNodeRef.current = onRequestAddNode;
  const onExecuteNodeRef = useRef(onExecuteNode);
  onExecuteNodeRef.current = onExecuteNode;
  const deleteEdgeRef = useRef<(id: string) => void>(() => {});
  const pendingConnectFrom = useRef<string | null>(null);
  const logsByNode = useMemo(
    () => buildNodeOverlayMap(runLogs),
    [runLogs],
  );
  const logsByNodeRef = useRef(logsByNode);
  logsByNodeRef.current = logsByNode;
  const logListRef = useRef(runLogs);
  logListRef.current = runLogs;

  const overlayFor = useCallback(
    (nodeId: string): Pick<WorkflowCanvasNodeData, "runStatus" | "durationMs"> | undefined => {
      const o = logsByNode.get(nodeId);
      if (!o) return undefined;
      return { runStatus: o.status, durationMs: o.durationMs };
    },
    [logsByNode],
  );

  const openNodeDetails = useCallback((id: string) => {
    const meta = metaRef.current.get(id) ?? null;
    onNodeSelectRef.current?.(meta);
  }, []);

  const toggleDisabled = useCallback((id: string) => {
    const existing = metaRef.current.get(id);
    if (!existing) return;
    const next: WorkflowNode = {
      ...existing,
      settings: {
        ...existing.settings,
        disabled: !existing.settings?.disabled,
      },
    };
    metaRef.current.set(id, next);
    setNodes((prev) =>
      prev.map((n) =>
        n.id === id
          ? toFlowNode(
              next,
              {
                onEdit: openNodeDetails,
                onExecute: (nid) => onExecuteNodeRef.current?.(nid),
                onToggleDisabled: toggleDisabled,
              },
              overlayFor(id),
            )
          : n,
      ),
    );
    toast.message(
      next.settings?.disabled ? "Node deactivated" : "Node activated",
    );
  }, [openNodeDetails, overlayFor]);

  const handlers = useMemo(
    () => ({
      onEdit: openNodeDetails,
      onExecute: (id: string) => onExecuteNodeRef.current?.(id),
      onToggleDisabled: toggleDisabled,
      onAddFromHandle: (id: string, sourceHandle: string) => {
        onRequestAddNodeRef.current?.({
          connectFrom: id,
          sourceHandle,
        });
      },
    }),
    [openNodeDetails, toggleDisabled],
  );
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  const initialNodes = useMemo(
    () =>
      (workflow?.nodes ?? []).map((n) =>
        toFlowNode(n, handlers, overlayFor(n.id)),
      ),
    [workflow, handlers, overlayFor],
  );

  const makeEdge = useCallback(
    (e: {
      id: string;
      source: string;
      target: string;
      sourceHandle?: string | null;
      targetHandle?: string | null;
    }): Edge => {
      const overlay = logsByNodeRef.current.get(e.source);
      const sourceLog = (logListRef.current ?? []).find(
        (l) => l.nodeId === e.source,
      );
      const executed = Boolean(overlay);
      const failed = overlay?.status === "failed";
      const itemCount = executed
        ? itemCountFromOutput(sourceLog?.output, e.sourceHandle)
        : undefined;
      const sourceMeta =
        metaRef.current.get(e.source) ?? nodesByIdRef.current.get(e.source);
      const connectionLabel = sourceMeta
        ? connectionLabelForNode(sourceMeta.type, sourceMeta.parameters)
        : undefined;
      const stroke = failed
        ? "var(--destructive)"
        : executed
          ? "var(--brand)"
          : "var(--canvas-edge)";
      return {
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle ?? undefined,
        targetHandle: e.targetHandle ?? undefined,
        type: "addable",
        selectable: false,
        focusable: false,
        style: { stroke, strokeWidth: executed ? 2 : 1.5 },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 16,
          height: 16,
          color: stroke,
        },
        data: {
          itemCount,
          executed,
          failed,
          connectionLabel,
          onAdd: readOnly
            ? undefined
            : (edgeId: string) => {
                const edge = edgesRef.current.find((x) => x.id === edgeId);
                if (edge) {
                  pendingConnectFrom.current = edge.source;
                  onRequestAddNodeRef.current?.({
                    connectFrom: edge.source,
                    sourceHandle: edge.sourceHandle ?? undefined,
                  });
                }
              },
          onDelete: readOnly
            ? undefined
            : (edgeId: string) => {
                deleteEdgeRef.current(edgeId);
              },
        } satisfies EdgeOverlayData,
      };
    },
    [readOnly],
  );

  const initialEdges: Edge[] = useMemo(
    () =>
      (workflow?.edges ?? []).map((e) =>
        makeEdge({
          id: e.id,
          source: e.source,
          target: e.target,
          sourceHandle: e.sourceHandle,
          targetHandle: e.targetHandle,
        }),
      ),
    [workflow, makeEdge],
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [edgeMenu, setEdgeMenu] = useState<{
    id: string;
    x: number;
    y: number;
  } | null>(null);
  const [nodeMenu, setNodeMenu] = useState<{
    id: string;
    x: number;
    y: number;
  } | null>(null);

  const closeMenus = useCallback(() => {
    setEdgeMenu(null);
    setNodeMenu(null);
  }, []);
  const edgesRef = useRef(edges);
  edgesRef.current = edges;
  const nodesRef = useRef(nodes);
  nodesRef.current = nodes;

  // Sync from server when switching workflows or after a successful save/reload.
  // Do NOT depend on overlayFor — that changes on every run-log tick and was
  // wiping freshly added (not-yet-saved) nodes off the canvas.
  useEffect(() => {
    const map = new Map<string, WorkflowNode>();
    for (const n of workflow?.nodes ?? []) map.set(n.id, { ...n });
    metaRef.current = map;
    const h = handlersRef.current;
    setNodes(
      (workflow?.nodes ?? []).map((n) =>
        toFlowNode(n, h, overlayFor(n.id)),
      ),
    );
    setEdges(
      (workflow?.edges ?? []).map((e) =>
        makeEdge({
          id: e.id,
          source: e.source,
          target: e.target,
          sourceHandle: e.sourceHandle,
          targetHandle: e.targetHandle,
        }),
      ),
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps -- sync only on server revision
  }, [workflow?._id, workflow?.updatedAt, workflow?.version, setNodes, setEdges, makeEdge]);

  // Re-apply execution overlay without wiping canvas positions / unsaved edits.
  useEffect(() => {
    const h = handlersRef.current;
    setNodes((prev) =>
      prev.map((n) => {
        const meta = metaRef.current.get(n.id);
        if (!meta) {
          const o = overlayFor(n.id);
          return {
            ...n,
            data: {
              ...n.data,
              runStatus: o?.runStatus,
              durationMs: o?.durationMs,
            },
          };
        }
        return {
          ...toFlowNode(meta, h, overlayFor(n.id)),
          position: n.position,
          selected: n.selected,
        };
      }),
    );
    setEdges((prev) =>
      prev.map((e) =>
        makeEdge({
          id: e.id,
          source: e.source,
          target: e.target,
          sourceHandle: e.sourceHandle,
          targetHandle: e.targetHandle,
        }),
      ),
    );
  }, [overlayFor, makeEdge, setNodes, setEdges]);

  useEffect(() => {
    if (!pendingAdd || !workflow?._id) return;
    if (pendingAdd.token === lastAddToken.current) return;
    lastAddToken.current = pendingAdd.token;
    const id = `n_${Date.now().toString(36)}_${shortId(5)}`;
    const offset = metaRef.current.size * 24;
    const parameters = defaultParameters(pendingAdd.type);
    if (pendingAdd.type === "webhook") {
      const path = String(parameters.path ?? "").trim();
      if (!path) parameters.path = shortId(12);
      if (!parameters.method) parameters.method = "POST";
      if (!parameters.authentication) parameters.authentication = "none";
      if (!parameters.respond) parameters.respond = "immediately";
    }
    if (pendingAdd.type === "apiEndpoint") {
      const path = String(parameters.path ?? "").trim();
      if (!path) parameters.path = `endpoint-${shortId(6)}`;
      if (!parameters.method) parameters.method = "GET";
      if (!parameters.auth) parameters.auth = "jwt";
    }
    const connectFrom =
      pendingAdd.connectFrom ?? pendingConnectFrom.current ?? undefined;
    const sourceHandle = pendingAdd.sourceHandle;
    pendingConnectFrom.current = null;

    let position = { x: 120 + offset, y: 120 + offset };
    if (connectFrom) {
      const src = nodesRef.current.find((n) => n.id === connectFrom);
      if (src) {
        position = { x: src.position.x + 240, y: src.position.y };
      }
    }

    const wfNode: WorkflowNode = {
      id,
      type: pendingAdd.type,
      name: pendingAdd.label,
      position,
      parameters,
    };
    metaRef.current.set(id, wfNode);

    const nextNodes = [
      ...nodesRef.current.map((n) => ({ ...n, selected: false })),
      { ...toFlowNode(wfNode, handlers, overlayFor(id)), selected: true },
    ];
    const nextEdges = connectFrom
      ? [
          ...edgesRef.current,
          makeEdge({
            id: `e_${connectFrom}_${id}_${sourceHandle ?? "main"}`,
            source: connectFrom,
            target: id,
            sourceHandle: sourceHandle ?? null,
          }),
        ]
      : edgesRef.current;

    setNodes(nextNodes);
    setEdges(nextEdges);
    nodesRef.current = nextNodes;
    edgesRef.current = nextEdges;

    openNodeDetails(id);
    toast.success(`Added ${pendingAdd.label}`);

    // Persist immediately so a later server sync cannot wipe the new node.
    void (async () => {
      try {
        await api(`/api/workflows/${workflow._id}`, {
          method: "PUT",
          body: JSON.stringify({
            name: workflow.name,
            enabled: workflow.enabled,
            nodes: nextNodes.map((n) => {
              const meta = metaRef.current.get(n.id);
              return {
                id: n.id,
                type: meta?.type ?? "manual",
                name: meta?.name ?? n.data.name,
                position: n.position,
                parameters: meta?.parameters ?? {},
                settings: meta?.settings,
              };
            }),
            edges: nextEdges.map((e) => ({
              id: e.id,
              source: e.source,
              target: e.target,
              sourceHandle: e.sourceHandle ?? undefined,
              targetHandle: e.targetHandle ?? undefined,
            })),
          }),
        });
        onSaved();
      } catch (err) {
        toast.error(
          err instanceof Error ? err.message : "Failed to save new node",
        );
      }
    })();
  }, [
    pendingAdd,
    workflow?._id,
    workflow?.name,
    workflow?.enabled,
    setNodes,
    setEdges,
    openNodeDetails,
    handlers,
    makeEdge,
    overlayFor,
    onSaved,
  ]);

  const onConnect = useCallback(
    (connection: Connection) =>
      setEdges((eds) =>
        addEdge(
          makeEdge({
            id: `e_${connection.source}_${connection.target}_${shortId(4)}`,
            source: connection.source!,
            target: connection.target!,
            sourceHandle: connection.sourceHandle,
            targetHandle: connection.targetHandle,
          }),
          eds,
        ),
      ),
    [setEdges, makeEdge],
  );

  const onNodesDelete: OnNodesDelete = useCallback((deleted) => {
    for (const n of deleted) {
      metaRef.current.delete(n.id);
    }
    onNodeSelectRef.current?.(null);
  }, []);

  const onEdgeContextMenu = useCallback(
    (event: ReactMouseEvent, edge: Edge) => {
      if (readOnly) return;
      event.preventDefault();
      event.stopPropagation();
      setNodeMenu(null);
      setEdgeMenu({ id: edge.id, x: event.clientX, y: event.clientY });
    },
    [readOnly],
  );

  const onNodeContextMenu = useCallback(
    (event: ReactMouseEvent, node: WorkflowCanvasFlowNode) => {
      if (readOnly) return;
      event.preventDefault();
      event.stopPropagation();
      setEdgeMenu(null);
      setNodes((prev) =>
        prev.map((n) => ({ ...n, selected: n.id === node.id })),
      );
      setNodeMenu({ id: node.id, x: event.clientX, y: event.clientY });
    },
    [readOnly, setNodes],
  );

  const deleteEdge = useCallback(
    (id: string) => {
      setEdges((prev) => prev.filter((e) => e.id !== id));
      setEdgeMenu(null);
    },
    [setEdges],
  );
  deleteEdgeRef.current = deleteEdge;

  const renameNode = useCallback(
    (id: string) => {
      const existing = metaRef.current.get(id);
      if (!existing) return;
      const nextName = window.prompt("Rename node", existing.name);
      if (nextName == null) return;
      const trimmed = nextName.trim();
      if (!trimmed || trimmed === existing.name) return;
      const next: WorkflowNode = { ...existing, name: trimmed };
      metaRef.current.set(id, next);
      setNodes((prev) =>
        prev.map((n) =>
          n.id === id
            ? toFlowNode(next, handlersRef.current, overlayFor(id))
            : n,
        ),
      );
      onNodeSelectRef.current?.(next);
      toast.success("Node renamed");
    },
    [overlayFor, setNodes],
  );

  const duplicateNode = useCallback(
    (id: string) => {
      const existing = metaRef.current.get(id);
      if (!existing) return;
      const newId = `n_${shortId(8)}`;
      const copy: WorkflowNode = {
        ...existing,
        id: newId,
        name: `${existing.name} copy`,
        position: {
          x: existing.position.x + 48,
          y: existing.position.y + 48,
        },
        parameters: { ...existing.parameters },
        settings: existing.settings ? { ...existing.settings } : undefined,
      };
      metaRef.current.set(newId, copy);
      setNodes((prev) => [
        ...prev.map((n) => ({ ...n, selected: false })),
        {
          ...toFlowNode(copy, handlersRef.current, overlayFor(newId)),
          selected: true,
        },
      ]);
      openNodeDetails(newId);
      toast.success("Node duplicated");
    },
    [openNodeDetails, overlayFor, setNodes],
  );

  const copyNode = useCallback(async (id: string) => {
    const existing = metaRef.current.get(id);
    if (!existing) return;
    const payload = JSON.stringify({
      kind: "botflow-node",
      node: {
        type: existing.type,
        name: existing.name,
        parameters: existing.parameters,
        settings: existing.settings,
      },
    });
    try {
      await navigator.clipboard.writeText(payload);
      toast.success("Node copied");
    } catch {
      toast.error("Could not copy node");
    }
  }, []);

  const selectAllNodes = useCallback(() => {
    setNodes((prev) => prev.map((n) => ({ ...n, selected: true })));
  }, [setNodes]);

  const clearNodeSelection = useCallback(() => {
    setNodes((prev) => prev.map((n) => ({ ...n, selected: false })));
    onNodeSelectRef.current?.(null);
  }, [setNodes]);

  // n8n-style shortcuts when a single canvas node is selected (not typing in a field).
  useEffect(() => {
    if (readOnly) return;
    function onKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable)
      ) {
        return;
      }
      const selected = nodesRef.current.filter((n) => n.selected);
      const mod = e.ctrlKey || e.metaKey;

      if (mod && e.key.toLowerCase() === "a") {
        e.preventDefault();
        selectAllNodes();
        return;
      }

      if (selected.length === 0) return;
      const id = selected[0]!.id;

      if (e.key === "Enter" && !mod) {
        e.preventDefault();
        closeMenus();
        openNodeDetails(id);
        return;
      }
      if (e.key === " " && !mod && selected.length === 1) {
        e.preventDefault();
        closeMenus();
        renameNode(id);
        return;
      }
      if (e.key.toLowerCase() === "d" && !mod && selected.length === 1) {
        e.preventDefault();
        toggleDisabled(id);
        return;
      }
      if (mod && e.key.toLowerCase() === "d" && selected.length === 1) {
        e.preventDefault();
        duplicateNode(id);
        return;
      }
      if (mod && e.key.toLowerCase() === "c" && selected.length === 1) {
        // Allow native copy when text is selected; otherwise copy the node.
        const sel = window.getSelection();
        if (sel && sel.toString().length > 0) return;
        e.preventDefault();
        void copyNode(id);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    readOnly,
    closeMenus,
    openNodeDetails,
    renameNode,
    toggleDisabled,
    duplicateNode,
    copyNode,
    selectAllNodes,
  ]);

  function updateNode(id: string, patch: NodeUpdatePatch): WorkflowNode | null {
    const existing = metaRef.current.get(id);
    if (!existing) return null;
    const next: WorkflowNode = {
      ...existing,
      ...patch,
      parameters: patch.parameters ?? existing.parameters,
      settings:
        patch.settings !== undefined
          ? { ...existing.settings, ...patch.settings }
          : existing.settings,
    };
    metaRef.current.set(id, next);
    setNodes((prev) =>
      prev.map((n) =>
        n.id === id ? toFlowNode(next, handlers, overlayFor(id)) : n,
      ),
    );
    // Refresh outbound edge labels (e.g. webhook method) after parameter edits.
    if (patch.parameters) {
      setEdges((prev) =>
        prev.map((e) =>
          e.source === id
            ? makeEdge({
                id: e.id,
                source: e.source,
                target: e.target,
                sourceHandle: e.sourceHandle,
                targetHandle: e.targetHandle,
              })
            : e,
        ),
      );
    }
    return next;
  }

  function deleteNode(id: string) {
    setNodes((prev) => prev.filter((n) => n.id !== id));
    setEdges((prev) =>
      prev.filter((e) => e.source !== id && e.target !== id),
    );
    metaRef.current.delete(id);
    onNodeSelectRef.current?.(null);
  }

  function getNode(id: string): WorkflowNode | null {
    return metaRef.current.get(id) ?? null;
  }

  async function save(opts?: FlowCanvasSaveOptions) {
    if (!workflow?._id) return;
    const body = {
      name: workflow.name,
      enabled: workflow.enabled,
      nodes: nodes.map((n) => {
        const meta = metaRef.current.get(n.id);
        const original = workflow.nodes.find((x) => x.id === n.id);
        return {
          id: n.id,
          type: meta?.type ?? original?.type ?? "manual",
          name: meta?.name ?? original?.name ?? n.data.name,
          position: n.position,
          parameters: meta?.parameters ?? original?.parameters ?? {},
          settings: meta?.settings ?? original?.settings,
        };
      }),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle ?? undefined,
        targetHandle: e.targetHandle ?? undefined,
      })),
    };
    await api(`/api/workflows/${workflow._id}`, {
      method: "PUT",
      body: JSON.stringify(body),
    });
    if (!opts?.silent) toast.success("Workflow saved");
    onSaved();
  }

  async function run() {
    if (!workflow?._id) return;
    const res = await api<{ run: { status: string; _id: string } }>(
      `/api/workflows/${workflow._id}/run`,
      { method: "POST", body: JSON.stringify({ input: { from: "ui" } }) },
    );
    toast.message(`Run ${res.run.status}`, { description: res.run._id });
    onSaved();
  }

  useImperativeHandle(
    ref,
    () => ({
      save: () => save(),
      run: () => run(),
      updateNode,
      deleteNode,
      getNode,
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- mirror latest canvas graph
    [nodes, edges, workflow, handlers],
  );

  if (!workflow) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--canvas)] text-sm text-muted-foreground">
        Select or create a workflow to view the canvas
      </div>
    );
  }

  const empty = nodes.length === 0;

  return (
    <div className="relative flex h-full flex-col bg-[var(--canvas)]">
      {!hideChrome ? (
        <div className="flex items-center gap-2 border-b bg-card px-3 py-2">
          <h2 className="text-sm font-semibold">{workflow.name}</h2>
          <div className="flex-1" />
          <button
            type="button"
            className="rounded-md border px-3 py-1.5 text-xs hover:bg-muted"
            onClick={() => void save()}
          >
            Save
          </button>
          <button
            type="button"
            className="rounded-md px-3 py-1.5 text-xs text-white"
            style={{ background: "var(--brand)" }}
            onClick={() => void run()}
          >
            Execute workflow
          </button>
        </div>
      ) : null}
      <div className="relative min-h-0 flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodesDelete={onNodesDelete}
          deleteKeyCode={["Backspace", "Delete"]}
          edgesFocusable={false}
          edgesSelectable={false}
          onEdgeClick={() => {
            // Keep edges interactive (xyflow marks non-selectable edges without
            // onClick as `.inactive` → pointer-events: none).
            closeMenus();
          }}
          onEdgeContextMenu={onEdgeContextMenu}
          onNodeContextMenu={onNodeContextMenu}
          onNodeClick={() => {
            closeMenus();
          }}
          onNodeDoubleClick={(_e, node) => {
            closeMenus();
            openNodeDetails(node.id);
          }}
          onPaneClick={() => {
            closeMenus();
            onNodeSelectRef.current?.(null);
          }}
          onDoubleClick={(event) => {
            const target = event.target as HTMLElement | null;
            if (!target) return;
            if (
              target.closest(".react-flow__node") ||
              target.closest(".react-flow__edge") ||
              target.closest(".react-flow__controls") ||
              target.closest(".react-flow__minimap")
            ) {
              return;
            }
            if (
              target.classList.contains("react-flow__pane") ||
              target.closest(".react-flow__pane")
            ) {
              onRequestAddNodeRef.current?.();
            }
          }}
          colorMode={colorMode}
          fitView
          proOptions={{ hideAttribution: true }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={20}
            size={1.35}
            color="var(--canvas-dot)"
          />
          <Controls showInteractive={false} position="bottom-left" />
          <MiniMap
            position="bottom-left"
            style={{ marginBottom: 48 }}
            pannable
            zoomable
            bgColor="var(--canvas)"
            maskColor={
              colorMode === "dark"
                ? "rgba(0, 0, 0, 0.55)"
                : "rgba(250, 250, 250, 0.65)"
            }
            nodeColor={colorMode === "dark" ? "#7a7a7a" : "#b8b8b8"}
            nodeStrokeColor="var(--brand)"
            nodeStrokeWidth={2}
            nodeBorderRadius={6}
          />
        </ReactFlow>

        {empty ? (
          <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
            <div className="pointer-events-auto flex flex-col items-center gap-3 rounded-xl border bg-card/95 px-8 py-6 shadow-lg backdrop-blur">
              <p className="text-sm text-muted-foreground">
                This workflow is empty
              </p>
              <Button
                size="lg"
                onClick={() =>
                  onRequestAddNodeRef.current?.({ triggersOnly: true })
                }
              >
                <Plus className="h-4 w-4" />
                Add first step
              </Button>
            </div>
          </div>
        ) : null}

        {edgeMenu || nodeMenu ? (
          <div
            className="fixed inset-0 z-40"
            aria-hidden
            onClick={closeMenus}
            onContextMenu={(e) => {
              e.preventDefault();
              closeMenus();
            }}
          />
        ) : null}

        {edgeMenu ? (
          <div
            role="menu"
            className="fixed z-50 min-w-44 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
            style={{ left: edgeMenu.x, top: edgeMenu.y }}
          >
            <button
              type="button"
              role="menuitem"
              className="flex w-full cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-destructive outline-hidden select-none hover:bg-destructive/10"
              onClick={() => deleteEdge(edgeMenu.id)}
            >
              <Trash2 className="size-4 shrink-0" />
              Delete connection
            </button>
          </div>
        ) : null}

        {nodeMenu ? (
          <NodeContextMenu
            x={nodeMenu.x}
            y={nodeMenu.y}
            disabled={Boolean(metaRef.current.get(nodeMenu.id)?.settings?.disabled)}
            onOpen={() => {
              openNodeDetails(nodeMenu.id);
              closeMenus();
            }}
            onExecute={() => {
              onExecuteNodeRef.current?.(nodeMenu.id);
              closeMenus();
            }}
            onRename={() => {
              const id = nodeMenu.id;
              closeMenus();
              // Defer so the menu unmounts before the prompt steals focus.
              queueMicrotask(() => renameNode(id));
            }}
            onToggleDisabled={() => {
              toggleDisabled(nodeMenu.id);
              closeMenus();
            }}
            onCopy={() => {
              void copyNode(nodeMenu.id);
              closeMenus();
            }}
            onDuplicate={() => {
              duplicateNode(nodeMenu.id);
              closeMenus();
            }}
            onSelectAll={() => {
              selectAllNodes();
              closeMenus();
            }}
            onClearSelection={() => {
              clearNodeSelection();
              closeMenus();
            }}
            onDelete={() => {
              deleteNode(nodeMenu.id);
              closeMenus();
            }}
          />
        ) : null}
      </div>
    </div>
  );
});
