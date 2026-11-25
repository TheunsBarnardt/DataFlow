import type { WorkflowNode, NodeType } from "@dataflow/shared";
import type { WorkflowItem } from "./items.js";
import type { ExpressionRunContext } from "./resolveExpressions.js";

export type { WorkflowItem, Item } from "./items.js";

export interface NodeContext {
  node: WorkflowNode;
  items: WorkflowItem[];
  /**
   * Items grouped by targetHandle (default "main").
   * Merge uses "0" / "1" (or first/second incoming edge when handles unset).
   */
  inputsByHandle: Record<string, WorkflowItem[]>;
  runInput: unknown;
  /** Sync env lookup; prefer getCredentialAsync for stored credentials. */
  getCredential: (key: string) => string | undefined;
  /** Async env + Mongo credential resolution (ObjectId or id.field). */
  getCredentialAsync: (key: string) => Promise<string | undefined>;
  /** Full decrypted credential payload by Mongo id. */
  getCredentialData: (credentialId: string) => Promise<Record<string, unknown>>;
  /** Resolve a parameter value against vars + optional item ($json / ={{ }}). */
  resolve: <T>(value: T, item?: WorkflowItem) => T;
  vars: Record<string, string>;
  /** Persistent per-node context (SplitInBatches cursor, etc.). */
  getNodeContext: <T = Record<string, unknown>>() => T;
  setNodeContext: (data: Record<string, unknown>) => void;
  /** Expression run store for $('NodeName'). */
  expressionCtx: ExpressionRunContext;
}

/** Handlers may return structured items or flat JSON; executor normalizes. */
export type NodeOutputItem = WorkflowItem | Record<string, unknown>;

export interface NodeResult {
  /** Primary / "main" output items. */
  items: NodeOutputItem[];
  /**
   * Named outputs for multi-output nodes (true/false, kept/discarded, cases).
   * When set, executor routes by edge.sourceHandle; `items` should match the
   * primary output (usually "main" or "true" / "kept").
   */
  outputs?: Record<string, NodeOutputItem[]>;
  /** @deprecated Prefer `outputs`. Single active branch name for legacy routing. */
  branch?: string;
  respond?: { statusCode: number; body: unknown };
  /**
   * When true, executor will re-queue this node after downstream loop
   * connections finish (SplitInBatches).
   */
  continueLoop?: boolean;
  /**
   * When set, executor parks the run as waiting until resumeAt or webhook resume.
   */
  wait?: {
    resumeAt?: string;
    resumeToken?: string;
    /** Remaining node id to continue from after wait */
    continueFromNodeId?: string;
  };
}

export type NodeHandler = (ctx: NodeContext) => Promise<NodeResult>;

export type NodeRegistry = Partial<Record<NodeType, NodeHandler>>;
