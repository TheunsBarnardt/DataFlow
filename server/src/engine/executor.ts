import type {
  WorkflowDoc,
  RunDoc,
  NodeRunLog,
  RunStatus,
  RunTrigger,
  NodeType,
} from "@dataflow/shared";
import { ObjectId } from "mongodb";
import { runs, workflows, toId, serialize, type Stored } from "../db.js";
import { recordAudit } from "../audit.js";
import { emitExternal } from "../export.js";
import { registry } from "./registry.js";
import { loadVarMap } from "./resolveVars.js";
import {
  resolveExpressionsInValue,
  type ExpressionRunContext,
  type RunStoreEntry,
} from "./resolveExpressions.js";
import {
  ensureItems,
  minutesFromItemLike,
  normalizeItems,
  type WorkflowItem,
} from "./items.js";
import { errorMessage } from "../errorMessage.js";
import type { NodeResult } from "./types.js";
import {
  resolveCredentialValue,
  resolveCredentialFields,
} from "../credentials/resolve.js";
import {
  registerWait,
  markRunWaiting,
} from "./waitResume.js";
import { config } from "../config.js";

/** Map RunTrigger → canvas node types that should seed the run (n8n-like). */
const TRIGGER_SEED_TYPES: Partial<Record<RunTrigger, NodeType[]>> = {
  manual: ["manual", "simulateTrigger"],
  step: ["manual", "simulateTrigger"],
  schedule: ["schedule", "interval"],
  interval: ["interval", "schedule"],
  webhook: ["webhook"],
  form: ["formTrigger"],
  sse: ["sseTrigger"],
  outlook: ["outlookTrigger"],
  teams: ["teamsTrigger"],
  whatsapp: ["whatsappTrigger"],
  rabbitmq: ["rabbitmqTrigger"],
  redis: ["redisTrigger"],
  localFile: ["localFileTrigger"],
  rss: ["rssFeedTrigger"],
  email: ["emailTriggerImap"],
  error: ["errorTrigger"],
  executeWorkflow: ["executeWorkflowTrigger"],
  activation: ["activationTrigger"],
  simulate: ["simulateTrigger", "manual"],
  mcp: ["mcpServerTrigger", "manual"],
  mcpServer: ["mcpServerTrigger"],
  evaluation: ["evaluationTrigger"],
  api: ["apiEndpoint"],
  agent: ["manual", "simulateTrigger"],
  n8n: ["n8nTrigger"],
  workflow: ["workflowTrigger"],
};

const ALL_TRIGGER_TYPES = new Set<NodeType>([
  "webhook",
  "schedule",
  "manual",
  "rabbitmqTrigger",
  "redisTrigger",
  "outlookTrigger",
  "teamsTrigger",
  "whatsappTrigger",
  "formTrigger",
  "errorTrigger",
  "localFileTrigger",
  "sseTrigger",
  "rssFeedTrigger",
  "emailTriggerImap",
  "executeWorkflowTrigger",
  "workflowTrigger",
  "n8nTrigger",
  "interval",
  "activationTrigger",
  "simulateTrigger",
  "mcpServerTrigger",
  "evaluationTrigger",
  "apiEndpoint",
]);

function seedNodeIdsForTrigger(
  nodes: WorkflowDoc["nodes"],
  incoming: Map<string, WorkflowDoc["edges"]>,
  trigger: RunTrigger,
): string[] {
  const roots = nodes.filter((n) => (incoming.get(n.id) ?? []).length === 0);
  const matchTypes = TRIGGER_SEED_TYPES[trigger];
  if (matchTypes?.length) {
    const matched = roots.filter((n) => matchTypes.includes(n.type));
    if (matched.length) return matched.map((n) => n.id);
  }
  // Fallback: non-trigger roots + any matching trigger type if present as non-root
  const nonTriggers = roots.filter((n) => !ALL_TRIGGER_TYPES.has(n.type));
  if (nonTriggers.length) return nonTriggers.map((n) => n.id);
  return roots.map((n) => n.id);
}

async function fanOutErrorWorkflows(failed: {
  workflowId: string;
  runId: string;
  workflowName: string;
  error: string;
  trigger: RunTrigger;
  input?: unknown;
}): Promise<void> {
  if (failed.trigger === "error") return;
  const list = await workflows()
    .find({ enabled: true, "nodes.type": "errorTrigger" })
    .toArray();
  for (const wf of list) {
    if (String(wf._id) === failed.workflowId) continue;
    try {
      await executeWorkflow({
        workflowId: String(wf._id),
        trigger: "error",
        input: {
          trigger: "error",
          sourceWorkflowId: failed.workflowId,
          sourceWorkflowName: failed.workflowName,
          sourceRunId: failed.runId,
          error: failed.error,
          sourceTrigger: failed.trigger,
          sourceInput: failed.input,
        },
      });
    } catch (err) {
      console.error("[errorTrigger] fan-out failed", err);
    }
  }
}

export interface ExecuteOptions {
  workflowId: string;
  trigger: RunDoc["trigger"];
  input?: unknown;
  actor?: string;
  /** Resume after Wait: treat this node as already completed with items. */
  continueFrom?: { nodeId: string; items: WorkflowItem[]; runId?: string };
}

export interface ExecuteSingleNodeOptions {
  workflowId: string;
  nodeId: string;
  input?: unknown;
  parameters?: Record<string, unknown>;
}

type OutputMap = Record<string, WorkflowItem[]>;

const MULTI_INPUT_TYPES = new Set(["merge", "compareDatasets"]);
const LOOP_TYPES = new Set(["splitInBatches"]);

function minutesFromLogs(logs: NodeRunLog[]): number {
  let total = 0;
  for (const log of logs) {
    if (log.status !== "completed" || log.output == null) continue;
    const out = log.output;
    if (Array.isArray(out)) {
      for (const item of out) total += minutesFromItemLike(item);
    } else if (typeof out === "object" && out) {
      if ("minutesSaved" in out || "json" in out) {
        total += minutesFromItemLike(out);
      } else if ("items" in out && Array.isArray((out as { items: unknown }).items)) {
        for (const item of (out as { items: unknown[] }).items) {
          total += minutesFromItemLike(item);
        }
      } else if ("outputs" in out) {
        const outs = (out as { outputs: OutputMap }).outputs;
        for (const list of Object.values(outs ?? {})) {
          for (const item of list) total += minutesFromItemLike(item);
        }
      }
    }
  }
  return total;
}

function getCredential(key: string): string | undefined {
  return process.env[key];
}

async function getCredentialAsync(key: string): Promise<string | undefined> {
  return resolveCredentialValue(key);
}

async function getCredentialData(
  credentialId: string,
): Promise<Record<string, unknown>> {
  return resolveCredentialFields(credentialId);
}

function resultToOutputMap(result: NodeResult): OutputMap {
  if (result.outputs && Object.keys(result.outputs).length) {
    const map: OutputMap = {};
    for (const [k, v] of Object.entries(result.outputs)) {
      map[k] = ensureItems(v);
    }
    if (!map.main) {
      map.main = ensureItems(result.items);
    }
    return map;
  }
  const main = ensureItems(result.items);
  if (result.branch) {
    return { main, [result.branch]: main };
  }
  return { main };
}

function pickOutput(
  map: OutputMap | undefined,
  sourceHandle?: string,
): WorkflowItem[] {
  if (!map) return [];
  const handle = sourceHandle || "main";
  if (map[handle]) return map[handle]!;
  if (handle === "main") return map.main ?? [];
  return [];
}

function withPairedItems(
  items: WorkflowItem[],
  sourceIndexBase = 0,
): WorkflowItem[] {
  return items.map((item, i) => ({
    ...item,
    pairedItem: item.pairedItem ?? { item: sourceIndexBase + i },
  }));
}

/**
 * Waiting / stack executor: nodes run when inputs are ready; Merge waits for
 * both branches; SplitInBatches can re-enter via continueLoop.
 */
export async function executeWorkflow(
  opts: ExecuteOptions,
): Promise<RunDoc & { _id: string }> {
  const wf = await workflows().findOne({ _id: toId(opts.workflowId) });
  if (!wf) throw new Error(`Workflow not found: ${opts.workflowId}`);

  const actor = opts.actor;
  const resumeRunId = opts.continueFrom?.runId
    ? toId(opts.continueFrom.runId)
    : null;
  let runInsert: Stored<RunDoc>;
  if (resumeRunId) {
    const existingRun = await runs().findOne({ _id: resumeRunId });
    if (!existingRun) throw new Error("Run not found for resume");
    await runs().updateOne(
      { _id: resumeRunId },
      {
        $set: { status: "running" as RunStatus },
        $unset: { wait: "" },
      },
    );
    runInsert = { ...existingRun, status: "running" };
  } else {
    runInsert = {
      _id: new ObjectId(),
      workflowId: String(wf._id),
      workflowName: wf.name,
      status: "running",
      trigger: opts.trigger,
      startedAt: new Date().toISOString(),
      input: opts.input,
      logs: [],
      actor,
    };
    await runs().insertOne(runInsert);
  }
  void recordAudit({
    action: "run.started",
    resourceType: "run",
    resourceId: String(runInsert._id),
    resourceName: wf.name,
    actor,
    details: { workflowId: String(wf._id), trigger: opts.trigger },
  });

  const nodeMap = new Map(wf.nodes.map((n) => [n.id, n]));
  const ids = new Set(wf.nodes.map((n) => n.id));
  const outgoing = new Map<string, WorkflowDoc["edges"]>();
  const incoming = new Map<string, WorkflowDoc["edges"]>();
  for (const id of ids) {
    outgoing.set(id, []);
    incoming.set(id, []);
  }
  for (const e of wf.edges) {
    if (!ids.has(e.source) || !ids.has(e.target)) continue;
    outgoing.get(e.source)!.push(e);
    incoming.get(e.target)!.push(e);
  }

  const outputMaps = new Map<string, OutputMap>();
  const completed = new Set<string>();
  const nodeContexts = new Map<string, Record<string, unknown>>();
  const runStore = new Map<string, RunStoreEntry>();
  const logs: NodeRunLog[] = resumeRunId
    ? [...((runInsert.logs as NodeRunLog[]) ?? [])]
    : [];
  const varMap = await loadVarMap();
  let respond: { statusCode: number; body: unknown } | undefined;
  let finalItems: WorkflowItem[] = normalizeItems(
    opts.continueFrom?.items ?? opts.input,
  );
  const maxSteps = Math.max(500, ids.size * 50);
  let steps = 0;

  const expressionCtxBase = (): ExpressionRunContext => ({
    vars: varMap,
    runStore,
    throwOnError: true,
  });

  function sourcesReady(nodeId: string): boolean {
    const edges = incoming.get(nodeId) ?? [];
    if (!edges.length) return true;
    const node = nodeMap.get(nodeId);
    if (node && MULTI_INPUT_TYPES.has(node.type)) {
      // Wait until every distinct source has completed (empty branch OK)
      const sources = new Set(edges.map((e) => e.source));
      for (const src of sources) {
        if (!completed.has(src)) return false;
      }
      return true;
    }
    // Normal: at least one upstream completed, and all upstreams that exist are done
    // (or none connected yet for triggers)
    let any = false;
    for (const e of edges) {
      if (!completed.has(e.source)) return false;
      any = true;
    }
    return any || edges.length === 0;
  }

  function collectInputs(nodeId: string): {
    items: WorkflowItem[];
    inputsByHandle: Record<string, WorkflowItem[]>;
  } {
    const edges = incoming.get(nodeId) ?? [];
    const inputsByHandle: Record<string, WorkflowItem[]> = {};
    let items: WorkflowItem[] = [];
    if (!edges.length) {
      items = withPairedItems(normalizeItems(opts.input));
      inputsByHandle.main = items;
      inputsByHandle["0"] = items;
      return { items, inputsByHandle };
    }
    let autoIndex = 0;
    for (const edge of edges) {
      const srcMap = outputMaps.get(edge.source);
      const chunk = withPairedItems(
        pickOutput(srcMap, edge.sourceHandle),
      );
      const th =
        edge.targetHandle && edge.targetHandle.length
          ? edge.targetHandle
          : String(autoIndex++);
      if (!inputsByHandle[th]) inputsByHandle[th] = [];
      inputsByHandle[th]!.push(...chunk);
      items.push(...chunk);
    }
    if (!inputsByHandle.main) inputsByHandle.main = items;
    return { items, inputsByHandle };
  }

  // Seed: only the trigger that fired (n8n-like), not every root node
  const queue: string[] = [];
  const queued = new Set<string>();
  function enqueue(id: string) {
    if (queued.has(id) || !ids.has(id)) return;
    // Allow re-queue for loop nodes even if completed once
    queued.add(id);
    queue.push(id);
  }

  if (opts.continueFrom) {
    const cf = opts.continueFrom;
    const items0 = withPairedItems(ensureItems(cf.items));
    outputMaps.set(cf.nodeId, { main: items0 });
    completed.add(cf.nodeId);
    const cfNode = nodeMap.get(cf.nodeId);
    if (cfNode) {
      runStore.set(cfNode.name, {
        nodeId: cf.nodeId,
        nodeName: cfNode.name,
        outputs: { main: items0 },
      });
    }
    finalItems = items0;
    for (const e of outgoing.get(cf.nodeId) ?? []) enqueue(e.target);
  } else {
    for (const id of seedNodeIdsForTrigger(wf.nodes, incoming, opts.trigger)) {
      enqueue(id);
    }
  }

  try {
    while (queue.length && steps < maxSteps) {
      steps += 1;
      const nodeId = queue.shift()!;
      queued.delete(nodeId);

      const rawNode = nodeMap.get(nodeId);
      if (!rawNode) continue;

      if (!sourcesReady(nodeId) && (incoming.get(nodeId) ?? []).length > 0) {
        // Wait — recheck later if something else progresses
        continue;
      }

      if (rawNode.settings?.disabled) {
        outputMaps.set(nodeId, { main: [] });
        completed.add(nodeId);
        runStore.set(rawNode.name, {
          nodeId,
          nodeName: rawNode.name,
          outputs: { main: [] },
        });
        logs.push({
          nodeId,
          nodeName: rawNode.name,
          status: "completed",
          startedAt: new Date().toISOString(),
          finishedAt: new Date().toISOString(),
          output: { skipped: true, reason: "disabled" },
        });
        for (const e of outgoing.get(nodeId) ?? []) enqueue(e.target);
        continue;
      }

      let { items, inputsByHandle } = collectInputs(nodeId);
      if (rawNode.settings?.executeOnce && items.length > 1) {
        items = items.slice(0, 1);
      }

      const exprCtx: ExpressionRunContext = {
        ...expressionCtxBase(),
        parameters: rawNode.parameters ?? {},
      };

      const resolve = <T>(value: T, item?: WorkflowItem): T => {
        const idx = item ? items.indexOf(item) : undefined;
        return resolveExpressionsInValue(value, varMap, item, items, {
          ...exprCtx,
          itemIndex: idx !== undefined && idx >= 0 ? idx : 0,
        });
      };

      // Resolve non-item expressions ($vars, $('Node'), $now) once; leave
      // `={{ $json… }}` intact so handlers can map per item like n8n.
      const node = {
        ...rawNode,
        parameters: resolve(rawNode.parameters ?? {}) as Record<string, unknown>,
      };

      const handler = registry[node.type];
      const startedAt = new Date().toISOString();
      if (!handler) {
        const log: NodeRunLog = {
          nodeId,
          nodeName: node.name,
          status: "failed",
          startedAt,
          finishedAt: new Date().toISOString(),
          error: `No handler for node type: ${node.type}`,
        };
        logs.push(log);
        throw new Error(log.error);
      }

      const maxAttempts = node.settings?.retryOnFail
        ? Math.max(1, Number(node.settings.maxTries ?? 3))
        : 1;
      const waitBetweenTries = Math.max(
        0,
        Number(node.settings?.waitBetweenTries ?? 1000),
      );
      let attempt = 0;
      let lastError: unknown;
      let result: NodeResult | undefined;

      while (attempt < maxAttempts) {
        attempt += 1;
        try {
          result = await handler({
            node,
            items,
            inputsByHandle,
            runInput: opts.input,
            getCredential,
            getCredentialAsync,
            getCredentialData,
            resolve,
            vars: varMap,
            getNodeContext: <T = Record<string, unknown>>() =>
              (nodeContexts.get(nodeId) ?? {}) as T,
            setNodeContext: (data) => {
              nodeContexts.set(nodeId, {
                ...(nodeContexts.get(nodeId) ?? {}),
                ...data,
              });
            },
            expressionCtx: exprCtx,
          });
          lastError = undefined;
          break;
        } catch (err) {
          lastError = err;
          if (attempt >= maxAttempts) break;
          if (waitBetweenTries > 0) {
            await new Promise((r) => setTimeout(r, waitBetweenTries));
          }
        }
      }

      if (lastError !== undefined) {
        const message = errorMessage(lastError);
        const onError = node.settings?.onError ?? "stopWorkflow";
        logs.push({
          nodeId,
          nodeName: node.name,
          status: "failed",
          startedAt,
          finishedAt: new Date().toISOString(),
          input: items,
          error: message,
        });
        if (onError === "continue") {
          outputMaps.set(nodeId, { main: items });
          completed.add(nodeId);
          runStore.set(node.name, {
            nodeId,
            nodeName: node.name,
            outputs: { main: items },
          });
          finalItems = items;
          for (const e of outgoing.get(nodeId) ?? []) enqueue(e.target);
          continue;
        }
        if (onError === "continueErrorOutput") {
          const errItems = [{ json: { error: message } }];
          const errMap: OutputMap = { main: [], error: errItems };
          outputMaps.set(nodeId, errMap);
          completed.add(nodeId);
          runStore.set(node.name, {
            nodeId,
            nodeName: node.name,
            outputs: errMap,
          });
          finalItems = errItems;
          for (const e of outgoing.get(nodeId) ?? []) {
            if ((e.sourceHandle || "main") === "error") enqueue(e.target);
          }
          continue;
        }
        throw lastError instanceof Error ? lastError : new Error(message);
      }

      let outMap = resultToOutputMap(result!);
      let outItems = outMap.main ?? ensureItems(result!.items);
      if (
        (!outItems || outItems.length === 0) &&
        node.settings?.alwaysOutputData
      ) {
        outItems = [{ json: {} }];
        outMap = { ...outMap, main: outItems };
      }
      outputMaps.set(nodeId, outMap);
      completed.add(nodeId);
      runStore.set(node.name, {
        nodeId,
        nodeName: node.name,
        outputs: outMap,
      });
      // Prefer last non-empty main
      if (outItems.length) finalItems = outItems;
      if (result!.respond) respond = result!.respond;

      logs.push({
        nodeId,
        nodeName: node.name,
        status: "completed",
        startedAt,
        finishedAt: new Date().toISOString(),
        input: items,
        output:
          result!.outputs || result!.branch
            ? {
                items: outItems,
                outputs: outMap,
                ...(result!.branch ? { branch: result!.branch } : {}),
              }
            : outItems,
      });

      if (result!.wait?.resumeToken) {
        const resumeToken = result!.wait.resumeToken;
        const resumeUrl = `${config.publicBaseUrl}/hooks/wait-resume/${String(runInsert._id)}?token=${resumeToken}`;
        registerWait({
          runId: String(runInsert._id),
          workflowId: String(wf._id),
          nodeId,
          resumeToken,
          items: outItems,
          resumeAt: result!.wait.resumeAt,
        });
        await markRunWaiting(runInsert._id, {
          nodeId,
          resumeToken,
          resumeAt: result!.wait.resumeAt,
          resumeUrl,
        });
        await runs().updateOne(
          { _id: runInsert._id },
          { $set: { logs, output: { waiting: true, resumeUrl, items: outItems } } },
        );
        return serialize({
          ...runInsert,
          status: "waiting" as RunStatus,
          logs,
          output: { waiting: true, resumeUrl, resumeAt: result!.wait.resumeAt },
        });
      }

      // Enqueue downstream by matching sourceHandle to outputs that have items
      // (or always for empty-ok paths like false branch)
      for (const e of outgoing.get(nodeId) ?? []) {
        const handle = e.sourceHandle || "main";
        const chunk = pickOutput(outMap, handle);
        // Enqueue if this output key exists in map (even if empty) or is main
        if (
          handle in outMap ||
          handle === "main" ||
          chunk.length > 0 ||
          !result!.outputs
        ) {
          enqueue(e.target);
        }
      }

      // SplitInBatches: re-queue self when more batches remain
      if (result!.continueLoop || (LOOP_TYPES.has(node.type) && result!.continueLoop !== false)) {
        const ctx = nodeContexts.get(nodeId) ?? {};
        if (ctx.hasMore === true) {
          // Mark incomplete so sourcesReady for dependents of next iteration works
          completed.delete(nodeId);
          enqueue(nodeId);
        }
      }
    }

    // Process any nodes that became ready after merges (second pass for stragglers)
    for (const id of ids) {
      if (!completed.has(id) && sourcesReady(id)) {
        enqueue(id);
      }
    }
    while (queue.length && steps < maxSteps) {
      // Same loop body would duplicate — re-run a simplified drain by recursion risk;
      // instead, if queue refills from above seed, process remaining once more via goto-like
      steps += 1;
      const nodeId = queue.shift()!;
      queued.delete(nodeId);
      if (completed.has(nodeId) && !LOOP_TYPES.has(nodeMap.get(nodeId)?.type ?? "")) {
        continue;
      }
      if (!sourcesReady(nodeId) && (incoming.get(nodeId) ?? []).length > 0) continue;

      const rawNode = nodeMap.get(nodeId);
      if (!rawNode || rawNode.settings?.disabled) {
        if (rawNode) {
          outputMaps.set(nodeId, { main: [] });
          completed.add(nodeId);
        }
        continue;
      }

      let { items, inputsByHandle } = collectInputs(nodeId);
      if (rawNode.settings?.executeOnce && items.length > 1) {
        items = items.slice(0, 1);
      }
      const exprCtx: ExpressionRunContext = {
        ...expressionCtxBase(),
        parameters: rawNode.parameters ?? {},
      };
      const resolve = <T>(value: T, item?: WorkflowItem): T =>
        resolveExpressionsInValue(value, varMap, item, items, {
          ...exprCtx,
          itemIndex: item ? Math.max(0, items.indexOf(item)) : 0,
        });
      // Leave $json expressions for per-item resolve (see main loop).
      const node = {
        ...rawNode,
        parameters: resolve(rawNode.parameters ?? {}) as Record<string, unknown>,
      };
      const handler = registry[node.type];
      if (!handler) continue;
      const startedAt = new Date().toISOString();
      try {
        const result = await handler({
          node,
          items,
          inputsByHandle,
          runInput: opts.input,
          getCredential,
          getCredentialAsync,
          getCredentialData,
          resolve,
          vars: varMap,
          getNodeContext: <T = Record<string, unknown>>() =>
            (nodeContexts.get(nodeId) ?? {}) as T,
          setNodeContext: (data) => {
            nodeContexts.set(nodeId, {
              ...(nodeContexts.get(nodeId) ?? {}),
              ...data,
            });
          },
          expressionCtx: exprCtx,
        });
        let outMap = resultToOutputMap(result);
        let outItems = outMap.main ?? ensureItems(result.items);
        if (
          (!outItems || outItems.length === 0) &&
          node.settings?.alwaysOutputData
        ) {
          outItems = [{ json: {} }];
          outMap = { ...outMap, main: outItems };
        }
        outputMaps.set(nodeId, outMap);
        completed.add(nodeId);
        runStore.set(node.name, {
          nodeId,
          nodeName: node.name,
          outputs: outMap,
        });
        if (outItems.length) finalItems = outItems;
        if (result.respond) respond = result.respond;
        logs.push({
          nodeId,
          nodeName: node.name,
          status: "completed",
          startedAt,
          finishedAt: new Date().toISOString(),
          input: items,
          output: outItems,
        });
        for (const e of outgoing.get(nodeId) ?? []) enqueue(e.target);
        const ctx = nodeContexts.get(nodeId) ?? {};
        if (result.continueLoop && ctx.hasMore === true) {
          completed.delete(nodeId);
          enqueue(nodeId);
        }
      } catch (err) {
        throw err;
      }
    }

    const status: RunStatus = "completed";
    const finishedAt = new Date().toISOString();
    const output = respond ?? finalItems;
    const fromNodes = minutesFromLogs(logs);
    const timeSavedMinutes =
      fromNodes > 0 ? fromNodes : Number(wf.timeSavedMinutes) || 0;
    await runs().updateOne(
      { _id: runInsert._id },
      { $set: { status, finishedAt, output, logs, timeSavedMinutes } },
    );
    const result = serialize({
      ...runInsert,
      status,
      finishedAt,
      output,
      logs,
      timeSavedMinutes,
    });
    void recordAudit({
      action: "run.completed",
      resourceType: "run",
      resourceId: result._id,
      resourceName: wf.name,
      actor,
      details: {
        workflowId: String(wf._id),
        trigger: opts.trigger,
        timeSavedMinutes,
      },
    });
    emitExternal("metric", {
      name: "dataflow.run.completed",
      workflowId: String(wf._id),
      workflowName: wf.name,
      runId: result._id,
      status,
      trigger: opts.trigger,
      durationMs:
        new Date(finishedAt).getTime() - new Date(runInsert.startedAt).getTime(),
      timeSavedMinutes,
    });
    return result;
  } catch (err) {
    const message = errorMessage(err);
    const finishedAt = new Date().toISOString();
    await runs().updateOne(
      { _id: runInsert._id },
      {
        $set: {
          status: "failed" as RunStatus,
          finishedAt,
          error: message,
          logs,
          timeSavedMinutes: 0,
        },
      },
    );
    const failedStatus: RunStatus = "failed";
    const result = serialize({
      ...runInsert,
      status: failedStatus,
      finishedAt,
      error: message,
      logs,
      timeSavedMinutes: 0,
    });
    void recordAudit({
      action: "run.failed",
      resourceType: "run",
      resourceId: result._id,
      resourceName: wf.name,
      actor,
      details: {
        workflowId: String(wf._id),
        trigger: opts.trigger,
        error: message,
      },
    });
    emitExternal("log", {
      level: "error",
      name: "dataflow.run.failed",
      workflowId: String(wf._id),
      workflowName: wf.name,
      runId: result._id,
      trigger: opts.trigger,
      error: message,
    });
    void fanOutErrorWorkflows({
      workflowId: String(wf._id),
      runId: result._id,
      workflowName: wf.name,
      error: message,
      trigger: opts.trigger,
      input: opts.input,
    });
    return result;
  }
}

/** Run a single node with provided (or empty) input — for Execute step in the editor. */
export async function executeSingleNode(
  opts: ExecuteSingleNodeOptions,
): Promise<{
  nodeId: string;
  input: WorkflowItem[];
  output: WorkflowItem[];
  outputs?: OutputMap;
  error?: string;
  runId?: string;
}> {
  const wf = await workflows().findOne({ _id: toId(opts.workflowId) });
  if (!wf) throw new Error(`Workflow not found: ${opts.workflowId}`);
  const node = wf.nodes.find((n) => n.id === opts.nodeId);
  if (!node) throw new Error(`Node not found: ${opts.nodeId}`);

  const items = normalizeItems(
    opts.input !== undefined ? opts.input : [{}],
  );

  const handler = registry[node.type];
  if (!handler) {
    throw new Error(`No handler for node type: ${node.type}`);
  }

  const parameters =
    opts.parameters !== undefined ? opts.parameters : (node.parameters ?? {});
  const varMap = await loadVarMap();
  const runStore = new Map<string, RunStoreEntry>();
  const nodeContexts = new Map<string, Record<string, unknown>>();
  const exprCtx: ExpressionRunContext = {
    vars: varMap,
    runStore,
    parameters,
    throwOnError: true,
  };
  const resolve = <T>(value: T, item?: WorkflowItem): T =>
    resolveExpressionsInValue(value, varMap, item, items, {
      ...exprCtx,
      itemIndex: item ? Math.max(0, items.indexOf(item)) : 0,
    });
  // Leave $json expressions for per-item resolve (n8n-style mapping).
  const resolvedNode = {
    ...node,
    parameters: resolve(parameters) as Record<string, unknown>,
  };

  const startedAt = new Date().toISOString();
  const runId = new ObjectId();

  try {
    const result = await handler({
      node: resolvedNode,
      items,
      inputsByHandle: { main: items, "0": items },
      runInput: opts.input,
      getCredential,
      getCredentialAsync,
      getCredentialData,
      resolve,
      vars: varMap,
      getNodeContext: <T = Record<string, unknown>>() =>
        (nodeContexts.get(node.id) ?? {}) as T,
      setNodeContext: (data) => {
        nodeContexts.set(node.id, {
          ...(nodeContexts.get(node.id) ?? {}),
          ...data,
        });
      },
      expressionCtx: exprCtx,
    });
    const outMap = resultToOutputMap(result);
    let outItems = outMap.main ?? ensureItems(result.items);
    if ((!outItems || outItems.length === 0) && node.settings?.alwaysOutputData) {
      outItems = [{ json: {} }];
    }
    const finishedAt = new Date().toISOString();
    const log: NodeRunLog = {
      nodeId: node.id,
      nodeName: node.name,
      status: "completed",
      startedAt,
      finishedAt,
      input: items,
      output: outItems,
    };
    await runs().insertOne({
      _id: runId,
      workflowId: String(wf._id),
      workflowName: wf.name,
      status: "completed",
      trigger: "step",
      startedAt,
      finishedAt,
      input: items,
      output: outItems,
      logs: [log],
    });
    return {
      nodeId: node.id,
      input: items,
      output: outItems,
      outputs: outMap,
      runId: String(runId),
    };
  } catch (err) {
    const message = errorMessage(err);
    const finishedAt = new Date().toISOString();
    const log: NodeRunLog = {
      nodeId: node.id,
      nodeName: node.name,
      status: "failed",
      startedAt,
      finishedAt,
      input: items,
      error: message,
    };
    await runs().insertOne({
      _id: runId,
      workflowId: String(wf._id),
      workflowName: wf.name,
      status: "failed",
      trigger: "step",
      startedAt,
      finishedAt,
      input: items,
      error: message,
      logs: [log],
      timeSavedMinutes: 0,
    });
    return {
      nodeId: node.id,
      input: items,
      output: [],
      error: message,
      runId: String(runId),
    };
  }
}

/** Continue a parked Wait node after timer or webhook resume. */
export async function resumeWorkflowAfterWait(opts: {
  runId: string;
  workflowId: string;
  nodeId: string;
  items: WorkflowItem[];
  resumeInput?: unknown;
}): Promise<RunDoc & { _id: string }> {
  const items =
    opts.resumeInput !== undefined
      ? normalizeItems(opts.resumeInput)
      : opts.items;
  return executeWorkflow({
    workflowId: opts.workflowId,
    trigger: "webhook",
    input: opts.resumeInput ?? opts.items,
    continueFrom: {
      nodeId: opts.nodeId,
      items,
      runId: opts.runId,
    },
  });
}
