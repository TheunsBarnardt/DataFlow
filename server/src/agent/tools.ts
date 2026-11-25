import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type {
  WorkflowDoc,
  WorkflowNode,
  WorkflowEdge,
  WebhookDoc,
} from "@dataflow/shared";
import {
  workflows,
  runs,
  webhooks,
  schedules,
  serialize,
  toId,
  type Stored,
} from "../db.js";
import { executeWorkflow } from "../engine/executor.js";
import { computeNext, refreshSchedules } from "../scheduler.js";
import { config } from "../config.js";
import { recordAudit } from "../audit.js";

export interface ToolResult {
  ok: boolean;
  data?: unknown;
  error?: string;
}

export const agentTools = [
  {
    name: "list_workflows",
    description: "List all workflows",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "get_workflow",
    description: "Get a workflow by id",
    inputSchema: {
      type: "object",
      properties: { workflowId: { type: "string" } },
      required: ["workflowId"],
    },
  },
  {
    name: "create_workflow",
    description:
      "Create a workflow with nodes and edges. Triggers: webhook, schedule, manual, rabbitmqTrigger, redisTrigger, outlookTrigger, teamsTrigger, whatsappTrigger, formTrigger, and other core triggers. Also HTTP, MSSQL, Dynamics, Outlook, files, Set/Filter/Code, and all Core Nodes from /api/nodes.",
    inputSchema: {
      type: "object",
      properties: {
        name: { type: "string" },
        description: { type: "string" },
        nodes: { type: "array" },
        edges: { type: "array" },
      },
      required: ["name", "nodes"],
    },
  },
  {
    name: "update_workflow",
    description: "Update an existing workflow",
    inputSchema: {
      type: "object",
      properties: {
        workflowId: { type: "string" },
        name: { type: "string" },
        nodes: { type: "array" },
        edges: { type: "array" },
        enabled: { type: "boolean" },
      },
      required: ["workflowId"],
    },
  },
  {
    name: "run_workflow",
    description: "Execute a workflow now",
    inputSchema: {
      type: "object",
      properties: {
        workflowId: { type: "string" },
        input: {},
      },
      required: ["workflowId"],
    },
  },
  {
    name: "get_run",
    description: "Get a run by id",
    inputSchema: {
      type: "object",
      properties: { runId: { type: "string" } },
      required: ["runId"],
    },
  },
  {
    name: "create_webhook",
    description:
      "Create an inbound hook for a workflow. kind: webhook | outlook | teams | whatsapp | form | sse",
    inputSchema: {
      type: "object",
      properties: {
        workflowId: { type: "string" },
        path: { type: "string" },
        method: { type: "string" },
        kind: { type: "string" },
      },
      required: ["workflowId"],
    },
  },
  {
    name: "create_schedule",
    description: "Create a schedule for a workflow (cron or intervalMinutes)",
    inputSchema: {
      type: "object",
      properties: {
        workflowId: { type: "string" },
        name: { type: "string" },
        cron: { type: "string" },
        intervalMinutes: { type: "number" },
      },
      required: ["workflowId"],
    },
  },
] as const;

function normalizeNodes(raw: unknown[]): WorkflowNode[] {
  return raw.map((n, i) => {
    const node = n as Partial<WorkflowNode>;
    return {
      id: node.id ?? `n${i + 1}`,
      type: (node.type as WorkflowNode["type"]) ?? "manual",
      name: node.name ?? node.type ?? `Node ${i + 1}`,
      position: node.position ?? { x: 120 + i * 220, y: 120 },
      parameters: node.parameters ?? {},
    };
  });
}

function normalizeEdges(raw: unknown[] | undefined, nodes: WorkflowNode[]): WorkflowEdge[] {
  if (raw?.length) {
    return raw.map((e, i) => {
      const edge = e as Partial<WorkflowEdge>;
      return {
        id: edge.id ?? `e${i + 1}`,
        source: String(edge.source),
        target: String(edge.target),
        sourceHandle: edge.sourceHandle,
        targetHandle: edge.targetHandle,
      };
    });
  }
  // Auto-chain if edges omitted
  const edges: WorkflowEdge[] = [];
  for (let i = 0; i < nodes.length - 1; i++) {
    edges.push({
      id: `e${i + 1}`,
      source: nodes[i].id,
      target: nodes[i + 1].id,
    });
  }
  return edges;
}

export async function callAgentTool(
  name: string,
  args: Record<string, unknown>,
): Promise<ToolResult> {
  try {
    switch (name) {
      case "list_workflows": {
        const list = await workflows().find({}).sort({ updatedAt: -1 }).toArray();
        return { ok: true, data: list.map(serialize) };
      }
      case "get_workflow": {
        const wf = await workflows().findOne({
          _id: toId(String(args.workflowId)),
        });
        if (!wf) return { ok: false, error: "Workflow not found" };
        return { ok: true, data: serialize(wf) };
      }
      case "create_workflow": {
        const nodes = normalizeNodes((args.nodes as unknown[]) ?? []);
        const edges = normalizeEdges(args.edges as unknown[] | undefined, nodes);
        const now = new Date().toISOString();
        const actor = "agent";
        const doc: Stored<WorkflowDoc> = {
          _id: new ObjectId(),
          name: String(args.name ?? "Agent flow"),
          description: args.description ? String(args.description) : undefined,
          enabled: true,
          version: 1,
          nodes,
          edges,
          createdBy: actor,
          updatedBy: actor,
          createdAt: now,
          updatedAt: now,
        };
        await workflows().insertOne(doc);
        const workflow = serialize(doc);
        await recordAudit({
          action: "workflow.created",
          resourceType: "workflow",
          resourceId: workflow._id,
          resourceName: workflow.name,
          actor,
        });
        return { ok: true, data: workflow };
      }
      case "update_workflow": {
        const id = toId(String(args.workflowId));
        const existing = await workflows().findOne({ _id: id });
        if (!existing) return { ok: false, error: "Workflow not found" };
        const actor = "agent";
        const nodes = args.nodes
          ? normalizeNodes(args.nodes as unknown[])
          : existing.nodes;
        const edges = args.edges
          ? normalizeEdges(args.edges as unknown[], nodes)
          : args.nodes
            ? normalizeEdges(undefined, nodes)
            : existing.edges;
        const $set = {
          name: args.name ? String(args.name) : existing.name,
          enabled:
            typeof args.enabled === "boolean" ? args.enabled : existing.enabled,
          nodes,
          edges,
          version: existing.version + 1,
          updatedBy: actor,
          updatedAt: new Date().toISOString(),
        };
        await workflows().updateOne({ _id: id }, { $set });
        const workflow = serialize({ ...existing, ...$set });
        await recordAudit({
          action: "workflow.updated",
          resourceType: "workflow",
          resourceId: workflow._id,
          resourceName: workflow.name,
          actor,
          details: { version: $set.version },
        });
        return { ok: true, data: workflow };
      }
      case "run_workflow": {
        const run = await executeWorkflow({
          workflowId: String(args.workflowId),
          trigger: "agent",
          input: args.input,
          actor: "agent",
        });
        return { ok: true, data: run };
      }
      case "get_run": {
        const run = await runs().findOne({ _id: toId(String(args.runId)) });
        if (!run) return { ok: false, error: "Run not found" };
        return { ok: true, data: serialize(run) };
      }
      case "create_webhook": {
        const path = String(args.path ?? nanoid(12));
        const secret = nanoid(24);
        const kind = (args.kind as WebhookDoc["kind"]) ?? "webhook";
        const doc = {
          _id: new ObjectId(),
          workflowId: String(args.workflowId),
          path,
          secret,
          method: (args.method as "POST") ?? "POST",
          kind,
        };
        await webhooks().insertOne(doc);
        return {
          ok: true,
          data: {
            ...serialize(doc),
            url: `http://${config.host}:${config.port}/hooks/${path}`,
            auth: `Bearer ${secret}`,
          },
        };
      }
      case "create_schedule": {
        const doc = {
          _id: new ObjectId(),
          workflowId: String(args.workflowId),
          name: String(args.name ?? "Schedule"),
          cron: args.cron ? String(args.cron) : undefined,
          intervalMinutes: args.intervalMinutes
            ? Number(args.intervalMinutes)
            : undefined,
          paused: false,
          nextRunAt: computeNext(
            args.cron ? String(args.cron) : undefined,
            args.intervalMinutes ? Number(args.intervalMinutes) : undefined,
          ),
        };
        await schedules().insertOne(doc);
        await refreshSchedules();
        return { ok: true, data: serialize(doc) };
      }
      default:
        return { ok: false, error: `Unknown tool: ${name}` };
    }
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}
