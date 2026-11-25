import type { FastifyInstance } from "fastify";
import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import { DAY_ONE_NODE_TYPES } from "@dataflow/shared";
import type {
  WorkflowDoc,
  ScheduleDoc,
  WebhookDoc,
  ThreadDoc,
  VariableDoc,
  CredentialDoc,
  RunTrigger,
} from "@dataflow/shared";
import {
  workflows,
  runs,
  schedules,
  webhooks,
  threads,
  mcpServers,
  variables,
  credentials,
  auditEvents,
  toId,
  serialize,
  type Stored,
} from "../db.js";
import { executeWorkflow, executeSingleNode } from "../engine/executor.js";
import { errorMessage } from "../errorMessage.js";
import { computeNext, refreshSchedules } from "../scheduler.js";
import { handleAgentMessage } from "../agent/chat.js";
import { config } from "../config.js";
import { recordAudit } from "../audit.js";
import { actorFromRequest } from "../auth/actor.js";
import { getInsights } from "../insights.js";
import { syncWebhooksFromWorkflow } from "../webhooks/sync.js";
import {
  syncSchedulesFromWorkflow,
  deleteSchedulesForWorkflow,
} from "../schedules/sync.js";
import { refreshTriggerListeners } from "../triggers/listeners.js";
import {
  syncApiEndpointsFromWorkflow,
  deleteApiEndpointsForWorkflow,
} from "../apis/sync.js";
import { registerApiProjectMgmtRoutes } from "./apiProjects.js";
import {
  awaitListen,
  cancelListen,
  captureTestWebhook,
  startWebhookListen,
} from "../webhooks/listen.js";

async function syncWorkflowSideEffects(
  workflow: Pick<WorkflowDoc, "nodes" | "enabled" | "name"> & { _id: string },
  opts?: { previouslyEnabled?: boolean },
): Promise<void> {
  const hooks = await syncWebhooksFromWorkflow(workflow);
  await syncSchedulesFromWorkflow(workflow);
  void refreshTriggerListeners();

  // Best-effort Graph subscriptions for Outlook / Teams triggers
  if (workflow.enabled) {
    void (async () => {
      try {
        const { ensureGraphSubscription } = await import(
          "../microsoft/subscriptions.js"
        );
        const { resolveCredentialValue, resolveCredentialFields } = await import(
          "../credentials/resolve.js"
        );
        for (const hook of hooks) {
          if (hook.kind !== "outlook" && hook.kind !== "teams") continue;
          const node = workflow.nodes.find((n) => n.id === hook.nodeId);
          const credentialId = String(node?.parameters.credentialId ?? "");
          const notificationUrl = `${config.publicBaseUrl}/hooks/${hook.path}`;
          await ensureGraphSubscription({
            kind: hook.kind,
            notificationUrl,
            credentialId: credentialId || undefined,
            getCredential: (k) => process.env[k],
            getCredentialAsync: resolveCredentialValue,
            getCredentialData: resolveCredentialFields,
          });
        }
      } catch (err) {
        console.warn("[graph-subscription]", err);
      }
    })();
  }

  const wasEnabled = opts?.previouslyEnabled;
  if (
    workflow.enabled &&
    wasEnabled === false &&
    workflow.nodes.some((n) => n.type === "activationTrigger")
  ) {
    void executeWorkflow({
      workflowId: workflow._id,
      trigger: "activation",
      input: { event: "activate", at: new Date().toISOString() },
    });
  } else if (
    workflow.enabled &&
    wasEnabled === true &&
    workflow.nodes.some(
      (n) =>
        n.type === "activationTrigger" &&
        String(n.parameters.event ?? "activate") === "update",
    )
  ) {
    void executeWorkflow({
      workflowId: workflow._id,
      trigger: "activation",
      input: { event: "update", at: new Date().toISOString() },
    });
  }
}

function triggerFromKind(kind: WebhookDoc["kind"] | undefined): RunTrigger {
  switch (kind) {
    case "outlook":
      return "outlook";
    case "teams":
      return "teams";
    case "whatsapp":
      return "whatsapp";
    case "form":
      return "form";
    case "sse":
      return "sse";
    default:
      return "webhook";
  }
}

function normalizeTags(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  const seen = new Set<string>();
  const out: string[] = [];
  for (const item of raw) {
    const tag = String(item ?? "")
      .trim()
      .slice(0, 40);
    if (!tag) continue;
    const key = tag.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(tag);
  }
  return out.slice(0, 20);
}

export async function registerApiRoutes(app: FastifyInstance) {
  app.get("/api/health", async () => ({
    ok: true,
    name: "DataFlow",
    time: new Date().toISOString(),
  }));

  app.get("/api/nodes", async () => ({ nodes: DAY_ONE_NODE_TYPES }));

  await registerApiProjectMgmtRoutes(app);

  // Insights + audit
  app.get("/api/insights", async (req) => {
    const q = req.query as { days?: string };
    const days = Number(q.days ?? 7);
    return { insights: await getInsights(Number.isFinite(days) ? days : 7) };
  });

  app.get("/api/audit", async (req) => {
    const q = req.query as { limit?: string; resourceId?: string };
    const filter = q.resourceId ? { resourceId: q.resourceId } : {};
    const list = await auditEvents()
      .find(filter)
      .sort({ at: -1 })
      .limit(Number(q.limit ?? 100))
      .toArray();
    return { events: list.map(serialize) };
  });

  app.delete("/api/audit", async (req) => {
    const q = req.query as { resourceId?: string };
    const filter = q.resourceId ? { resourceId: q.resourceId } : {};
    const res = await auditEvents().deleteMany(filter);
    return { ok: true, deletedCount: res.deletedCount };
  });

  // Variables (project-level)
  app.get("/api/variables", async () => {
    const list = await variables().find({}).sort({ key: 1 }).toArray();
    return { variables: list.map(serialize) };
  });

  app.post<{ Body: Partial<VariableDoc> }>("/api/variables", async (req, reply) => {
    const key = String(req.body.key ?? "").trim();
    if (!key) return reply.code(400).send({ error: "key required" });
    const now = new Date().toISOString();
    const existing = await variables().findOne({ key });
    if (existing) {
      return reply.code(409).send({ error: "Variable key already exists" });
    }
    const doc: Stored<VariableDoc> = {
      _id: new ObjectId(),
      key,
      value: String(req.body.value ?? ""),
      type: (req.body.type as VariableDoc["type"]) ?? "string",
      description: req.body.description,
      createdAt: now,
      updatedAt: now,
    };
    await variables().insertOne(doc);
    return { variable: serialize(doc) };
  });

  app.put<{ Params: { id: string }; Body: Partial<VariableDoc> }>(
    "/api/variables/:id",
    async (req, reply) => {
      const existing = await variables().findOne({ _id: toId(req.params.id) });
      if (!existing) return reply.code(404).send({ error: "Not found" });
      const key = req.body.key !== undefined ? String(req.body.key).trim() : existing.key;
      if (!key) return reply.code(400).send({ error: "key required" });
      if (key !== existing.key) {
        const clash = await variables().findOne({ key });
        if (clash) return reply.code(409).send({ error: "Variable key already exists" });
      }
      const $set = {
        key,
        value: req.body.value !== undefined ? String(req.body.value) : existing.value,
        type: (req.body.type as VariableDoc["type"]) ?? existing.type,
        description:
          req.body.description !== undefined ? req.body.description : existing.description,
        updatedAt: new Date().toISOString(),
      };
      await variables().updateOne({ _id: existing._id }, { $set });
      return { variable: serialize({ ...existing, ...$set }) };
    },
  );

  app.delete<{ Params: { id: string } }>("/api/variables/:id", async (req, reply) => {
    const res = await variables().deleteOne({ _id: toId(req.params.id) });
    if (!res.deletedCount) return reply.code(404).send({ error: "Not found" });
    return { ok: true };
  });

  // Credentials — secrets encrypted-at-rest lightly (base64 of JSON; NOT real crypto).
  // List/create responses never include `data`.
  function encryptCredentialData(data: unknown): string {
    return Buffer.from(JSON.stringify(data ?? {}), "utf8").toString("base64");
  }

  app.get("/api/credentials", async () => {
    const list = await credentials().find({}).sort({ name: 1 }).toArray();
    return {
      credentials: list.map((doc) => ({
        id: String(doc._id),
        name: doc.name,
        type: doc.type,
      })),
    };
  });

  app.post<{
    Body: { name?: string; type?: string; data?: unknown };
  }>("/api/credentials", async (req, reply) => {
    const name = String(req.body?.name ?? "").trim();
    const type = String(req.body?.type ?? "").trim();
    if (!name) return reply.code(400).send({ error: "name required" });
    if (!type) return reply.code(400).send({ error: "type required" });
    const now = new Date().toISOString();
    const doc: Stored<CredentialDoc> = {
      _id: new ObjectId(),
      name,
      type,
      data: encryptCredentialData(req.body?.data ?? {}),
      createdAt: now,
      updatedAt: now,
    };
    await credentials().insertOne(doc);
    return {
      credential: { id: String(doc._id), name: doc.name, type: doc.type },
    };
  });

  app.delete<{ Params: { id: string } }>("/api/credentials/:id", async (req, reply) => {
    const res = await credentials().deleteOne({ _id: toId(req.params.id) });
    if (!res.deletedCount) return reply.code(404).send({ error: "Not found" });
    return { ok: true };
  });

  // Workflows
  app.get("/api/workflows", async () => {
    const list = await workflows().find({}).sort({ updatedAt: -1 }).toArray();
    return { workflows: list.map(serialize) };
  });

  app.get<{ Params: { id: string } }>("/api/workflows/:id", async (req, reply) => {
    const wf = await workflows().findOne({ _id: toId(req.params.id) });
    if (!wf) return reply.code(404).send({ error: "Not found" });
    return { workflow: serialize(wf) };
  });

  app.post<{ Body: Partial<WorkflowDoc> }>("/api/workflows", async (req) => {
    const now = new Date().toISOString();
    const actor = actorFromRequest(req);
    const nodes = [...(req.body.nodes ?? [])];
    const doc: Stored<WorkflowDoc> = {
      _id: new ObjectId(),
      name: req.body.name ?? "Untitled flow",
      description: req.body.description,
      enabled: req.body.enabled ?? true,
      version: 1,
      nodes,
      edges: req.body.edges ?? [],
      tags: normalizeTags(req.body.tags),
      timeSavedMinutes:
        req.body.timeSavedMinutes !== undefined
          ? Number(req.body.timeSavedMinutes)
          : undefined,
      kind: req.body.kind ?? "workflow",
      apiProjectId: req.body.apiProjectId,
      createdBy: actor,
      updatedBy: actor,
      ownerId: req.user?.id,
      visibility:
        req.body.kind === "apiEndpoint" || req.body.apiProjectId
          ? "shared"
          : "personal",
      createdAt: now,
      updatedAt: now,
    };
    await syncWorkflowSideEffects({
      _id: String(doc._id),
      nodes: doc.nodes,
      enabled: doc.enabled,
      name: doc.name,
    });
    await syncApiEndpointsFromWorkflow({
      _id: String(doc._id),
      nodes: doc.nodes,
      enabled: doc.enabled,
      kind: doc.kind,
      apiProjectId: doc.apiProjectId,
    });
    await workflows().insertOne(doc);
    const workflow = serialize(doc);
    await recordAudit({
      action: "workflow.created",
      resourceType: "workflow",
      resourceId: workflow._id,
      resourceName: workflow.name,
      actor,
    });
    return { workflow };
  });

  app.put<{ Params: { id: string }; Body: Partial<WorkflowDoc> }>(
    "/api/workflows/:id",
    async (req, reply) => {
      const existing = await workflows().findOne({ _id: toId(req.params.id) });
      if (!existing) return reply.code(404).send({ error: "Not found" });
      const actor = actorFromRequest(req);
      const nodes = req.body.nodes ? [...req.body.nodes] : [...existing.nodes];
      const enabled = req.body.enabled ?? existing.enabled;
      const kind = req.body.kind ?? existing.kind ?? "workflow";
      const apiProjectId =
        req.body.apiProjectId !== undefined
          ? req.body.apiProjectId
          : existing.apiProjectId;
      const name = req.body.name ?? existing.name;
      await syncWorkflowSideEffects(
        {
          _id: String(existing._id),
          nodes,
          enabled,
          name,
        },
        { previouslyEnabled: existing.enabled },
      );
      await syncApiEndpointsFromWorkflow({
        _id: String(existing._id),
        nodes,
        enabled,
        kind,
        apiProjectId,
      });
      const $set = {
        name,
        description: req.body.description ?? existing.description,
        enabled,
        nodes,
        edges: req.body.edges ?? existing.edges,
        tags:
          req.body.tags !== undefined
            ? normalizeTags(req.body.tags)
            : (existing.tags ?? []),
        timeSavedMinutes:
          req.body.timeSavedMinutes !== undefined
            ? Number(req.body.timeSavedMinutes)
            : existing.timeSavedMinutes,
        kind,
        apiProjectId,
        version: existing.version + 1,
        updatedBy: actor,
        updatedAt: new Date().toISOString(),
      };
      await workflows().updateOne({ _id: existing._id }, { $set });
      const workflow = serialize({ ...existing, ...$set });
      await recordAudit({
        action: "workflow.updated",
        resourceType: "workflow",
        resourceId: workflow._id,
        resourceName: workflow.name,
        actor,
        details: { version: $set.version },
      });
      return { workflow };
    },
  );

  app.delete<{ Params: { id: string } }>("/api/workflows/:id", async (req, reply) => {
    const existing = await workflows().findOne({ _id: toId(req.params.id) });
    if (!existing) return reply.code(404).send({ error: "Not found" });
    const actor = actorFromRequest(req);
    const res = await workflows().deleteOne({ _id: existing._id });
    if (!res.deletedCount) return reply.code(404).send({ error: "Not found" });
    await deleteApiEndpointsForWorkflow(String(existing._id));
    await deleteSchedulesForWorkflow(String(existing._id));
    await webhooks().deleteMany({ workflowId: String(existing._id) });
    void refreshTriggerListeners();
    await recordAudit({
      action: "workflow.deleted",
      resourceType: "workflow",
      resourceId: String(existing._id),
      resourceName: existing.name,
      actor,
    });
    return { ok: true };
  });

  app.post<{ Params: { id: string }; Body: { input?: unknown; trigger?: string } }>(
    "/api/workflows/:id/run",
    async (req, reply) => {
      try {
        const run = await executeWorkflow({
          workflowId: req.params.id,
          trigger: (req.body?.trigger as "manual") ?? "manual",
          input: req.body?.input,
          actor: actorFromRequest(req),
        });
        return { run };
      } catch (err) {
        return reply.code(400).send({
          error: errorMessage(err),
        });
      }
    },
  );

  app.post<{
    Params: { id: string };
    Body: {
      nodeId: string;
      input?: unknown;
      parameters?: Record<string, unknown>;
    };
  }>("/api/workflows/:id/execute-node", async (req, reply) => {
    try {
      const result = await executeSingleNode({
        workflowId: req.params.id,
        nodeId: req.body.nodeId,
        input: req.body.input,
        parameters: req.body.parameters,
      });
      return result;
    } catch (err) {
      return reply.code(400).send({
        error: errorMessage(err),
      });
    }
  });

  // Runs
  app.get("/api/runs", async (req) => {
    const q = req.query as { workflowId?: string; limit?: string };
    const filter = q.workflowId ? { workflowId: q.workflowId } : {};
    const list = await runs()
      .find(filter)
      .sort({ startedAt: -1 })
      .limit(Number(q.limit ?? 50))
      .toArray();
    return { runs: list.map(serialize) };
  });

  app.delete("/api/runs", async (req) => {
    const q = req.query as { workflowId?: string };
    const filter = q.workflowId ? { workflowId: q.workflowId } : {};
    const res = await runs().deleteMany(filter);
    return { ok: true, deletedCount: res.deletedCount };
  });

  app.get<{ Params: { id: string } }>("/api/runs/:id", async (req, reply) => {
    const run = await runs().findOne({ _id: toId(req.params.id) });
    if (!run) return reply.code(404).send({ error: "Not found" });
    return { run: serialize(run) };
  });

  // Schedules
  app.get("/api/schedules", async () => {
    const list = await schedules().find({}).toArray();
    return { schedules: list.map(serialize) };
  });

  app.post<{ Body: Partial<ScheduleDoc> }>("/api/schedules", async (req) => {
    const doc: Stored<ScheduleDoc> = {
      _id: new ObjectId(),
      workflowId: String(req.body.workflowId),
      name: req.body.name ?? "Schedule",
      cron: req.body.cron,
      intervalMinutes: req.body.intervalMinutes,
      paused: req.body.paused ?? false,
      nextRunAt: computeNext(req.body.cron, req.body.intervalMinutes),
    };
    await schedules().insertOne(doc);
    await refreshSchedules();
    return { schedule: serialize(doc) };
  });

  app.patch<{ Params: { id: string }; Body: Partial<ScheduleDoc> }>(
    "/api/schedules/:id",
    async (req, reply) => {
      const existing = await schedules().findOne({ _id: toId(req.params.id) });
      if (!existing) return reply.code(404).send({ error: "Not found" });
      const $set = {
        name: req.body.name ?? existing.name,
        cron: req.body.cron ?? existing.cron,
        intervalMinutes: req.body.intervalMinutes ?? existing.intervalMinutes,
        paused: req.body.paused ?? existing.paused,
        nextRunAt: computeNext(
          req.body.cron ?? existing.cron,
          req.body.intervalMinutes ?? existing.intervalMinutes,
        ),
      };
      await schedules().updateOne({ _id: existing._id }, { $set });
      await refreshSchedules();
      return { schedule: serialize({ ...existing, ...$set }) };
    },
  );

  // Webhooks registry
  app.get("/api/webhooks", async () => {
    const list = await webhooks().find({}).toArray();
    return { webhooks: list.map(serialize) };
  });

  app.post<{ Body: Partial<WebhookDoc> }>("/api/webhooks", async (req) => {
    const path = req.body.path ?? nanoid(12);
    const doc: Stored<WebhookDoc> = {
      _id: new ObjectId(),
      workflowId: String(req.body.workflowId),
      nodeId: req.body.nodeId,
      path,
      secret: req.body.secret ?? nanoid(24),
      method: req.body.method ?? "POST",
      authentication: req.body.authentication ?? "headerBearer",
      kind: req.body.kind ?? "webhook",
    };
    await webhooks().insertOne(doc);
    return {
      webhook: serialize(doc),
      url: `${config.publicBaseUrl}/hooks/${doc.path}`,
      testUrl: `${config.publicBaseUrl}/hooks-test/${doc.path}`,
    };
  });

  app.post<{
    Body: {
      workflowId: string;
      path: string;
      method?: string;
      wait?: boolean;
    };
  }>("/api/webhooks/listen", async (req, reply) => {
    const path = String(req.body.path ?? "").trim();
    const workflowId = String(req.body.workflowId ?? "").trim();
    if (!path || !workflowId) {
      return reply.code(400).send({ error: "workflowId and path required" });
    }
    const started = startWebhookListen({
      workflowId,
      path,
      method: req.body.method,
    });
    const testUrl = `${config.publicBaseUrl}/hooks-test/${path}`;
    if (!req.body.wait) {
      return {
        listening: true,
        testUrl,
        expiresAt: started.expiresAt,
        waitMs: started.waitMs,
      };
    }
    try {
      const event = await awaitListen(path);
      return { listening: false, testUrl, event };
    } catch (err) {
      return reply.code(408).send({
        error: errorMessage(err),
        testUrl,
      });
    }
  });

  app.delete<{ Body: { path: string } }>("/api/webhooks/listen", async (req) => {
    const path = String(req.body?.path ?? "").trim();
    return { cancelled: path ? cancelListen(path) : false };
  });

  // Test inbound — captures payload for Listen for test event (no full run)
  app.all<{ Params: { path: string } }>("/hooks-test/:path", async (req, reply) => {
    const captured = captureTestWebhook(req.params.path, {
      method: req.method,
      headers: req.headers as Record<string, unknown>,
      query: req.query,
      body: req.body,
    });
    if (!captured) {
      return reply.code(404).send({
        error: "No active listen session for this path. Click Listen for test event first.",
      });
    }
    return reply.code(200).send({
      ok: true,
      message: "Test event received",
      event: captured,
    });
  });

  // Wait node resume webhook
  app.all<{
    Params: { runId: string };
    Querystring: { token?: string };
  }>("/hooks/wait-resume/:runId", async (req, reply) => {
    const token =
      req.query.token ||
      (typeof (req.body as { token?: string } | undefined)?.token === "string"
        ? (req.body as { token: string }).token
        : "");
    if (!token) return reply.code(400).send({ error: "token required" });
    const { resumeWait } = await import("../engine/waitResume.js");
    const result = await resumeWait(req.params.runId, token, {
      method: req.method,
      headers: req.headers,
      query: req.query,
      body: req.body,
    });
    if (!result.ok) {
      return reply.code(400).send({ error: result.error });
    }
    return { ok: true, runId: result.runId };
  });

  // Inbound hooks — webhook + Outlook / Teams / WhatsApp / Form / SSE
  app.all<{ Params: { path: string } }>("/hooks/:path", async (req, reply) => {
    const hook = await webhooks().findOne({ path: req.params.path });
    if (!hook) return reply.code(404).send({ error: "Unknown webhook" });

    const wf = await workflows().findOne({ _id: toId(hook.workflowId) });
    if (!wf) return reply.code(404).send({ error: "Workflow not found" });
    if (!wf.enabled) {
      return reply
        .code(409)
        .send({ error: "Workflow is disabled. Enable it to use the production URL." });
    }

    const q = req.query as Record<string, string | undefined>;

    // WhatsApp / Meta webhook verification
    if (
      req.method === "GET" &&
      (hook.kind === "whatsapp" || q["hub.mode"] === "subscribe")
    ) {
      const mode = q["hub.mode"];
      const token = q["hub.verify_token"];
      const challenge = q["hub.challenge"];
      const expected = config.whatsappVerifyToken || hook.secret;
      if (mode === "subscribe" && token === expected && challenge) {
        return reply.type("text/plain").send(challenge);
      }
    }

    // Microsoft Graph subscription validation
    if (req.method === "POST" && (hook.kind === "outlook" || hook.kind === "teams")) {
      const body = req.body as { validationToken?: string } | undefined;
      const validation =
        (typeof q.validationToken === "string" ? q.validationToken : undefined) ||
        body?.validationToken;
      if (validation) {
        return reply.type("text/plain").send(validation);
      }
    }

    const auth = req.headers.authorization;
    const bearer = auth?.startsWith("Bearer ") ? auth.slice(7) : undefined;
    const providerHook =
      hook.kind === "whatsapp" ||
      hook.kind === "outlook" ||
      hook.kind === "teams" ||
      hook.kind === "form";
    const mustAuth =
      !providerHook && (hook.authentication ?? "none") === "headerBearer";
    if (mustAuth && bearer !== hook.secret && q.secret !== hook.secret) {
      return reply.code(401).send({ error: "Unauthorized" });
    }

    const webhookNode = wf.nodes.find(
      (n) =>
        n.id === hook.nodeId ||
        (n.type === "webhook" && String(n.parameters.path ?? "") === hook.path) ||
        (["formTrigger", "sseTrigger", "outlookTrigger", "teamsTrigger", "whatsappTrigger"].includes(
          n.type,
        ) &&
          String(n.parameters.path ?? "") === hook.path),
    );
    const respondMode = String(webhookNode?.parameters.respond ?? "immediately");
    const responseCode = Number(webhookNode?.parameters.responseCode ?? 200);

    const run = await executeWorkflow({
      workflowId: hook.workflowId,
      trigger: triggerFromKind(hook.kind),
      input: {
        kind: hook.kind ?? "webhook",
        method: req.method,
        headers: req.headers,
        query: req.query,
        body: req.body,
      },
    });

    if (hook.kind === "sse") {
      reply.raw.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      });
      const payload = JSON.stringify({
        runId: run._id,
        status: run.status,
        output: run.output,
      });
      reply.raw.write(`event: run\ndata: ${payload}\n\n`);
      reply.raw.end();
      return;
    }

    if (hook.kind === "form" && respondMode === "immediately") {
      return reply
        .type("text/html")
        .code(responseCode || 200)
        .send(
          `<!doctype html><html><body><h1>Submitted</h1><p>Run ${run._id}</p></body></html>`,
        );
    }

    if (respondMode === "immediately") {
      return reply.code(responseCode || 200).send({
        message: "Workflow got started",
        runId: run._id,
      });
    }

    const respond = (run.output as { statusCode?: number; body?: unknown }) ?? {};
    if (respond.statusCode && "body" in respond) {
      return reply.code(respond.statusCode).send(respond.body);
    }
    if (respondMode === "lastNode") {
      return reply.code(responseCode || 200).send(run.output ?? { ok: true });
    }
    // Graph expects 202 for change notifications
    if (hook.kind === "outlook" || hook.kind === "teams") {
      return reply.code(202).send({ ok: true, runId: run._id });
    }
    return { ok: true, runId: run._id, status: run.status };
  });

  // Chat / agent
  app.get("/api/threads", async () => {
    const list = await threads().find({}).sort({ updatedAt: -1 }).toArray();
    return {
      threads: list.map((t) =>
        serialize({
          ...t,
          messages: t.messages?.slice(-1) ?? [],
        }),
      ),
    };
  });

  app.post("/api/threads", async (_req, reply) => {
    const now = new Date().toISOString();
    const doc: Stored<ThreadDoc> = {
      _id: new ObjectId(),
      title: "New chat",
      messages: [],
      createdAt: now,
      updatedAt: now,
    };
    await threads().insertOne(doc);
    return reply.send({ thread: serialize(doc) });
  });

  app.get<{ Params: { id: string } }>("/api/threads/:id", async (req, reply) => {
    const t = await threads().findOne({ _id: toId(req.params.id) });
    if (!t) return reply.code(404).send({ error: "Not found" });
    return { thread: serialize(t) };
  });

  app.post<{ Params: { id: string }; Body: { content: string } }>(
    "/api/threads/:id/messages",
    async (req, reply) => {
      try {
        const result = await handleAgentMessage(req.params.id, req.body.content);
        return result;
      } catch (err) {
        return reply.code(400).send({
          error: errorMessage(err),
        });
      }
    },
  );

  // MCP server configs
  app.get("/api/mcp-servers", async () => {
    const list = await mcpServers().find({}).toArray();
    return { servers: list.map(serialize) };
  });

  app.post("/api/mcp-servers", async (req) => {
    const body = req.body as Record<string, unknown>;
    const doc = {
      _id: new ObjectId(),
      name: String(body.name ?? "mcp"),
      transport: (body.transport as "stdio" | "http") ?? "stdio",
      command: body.command as string | undefined,
      args: body.args as string[] | undefined,
      url: body.url as string | undefined,
      env: body.env as Record<string, string> | undefined,
      enabled: body.enabled !== false,
    };
    await mcpServers().insertOne(doc);
    return { server: serialize(doc) };
  });

  /** Dataverse entity / column loadOptions (parity with n8n-nodes-dataverse-auth). */
  app.get<{
    Querystring: {
      resource?: string;
      q?: string;
      credentialId?: string;
      entityName?: string;
      baseUrl?: string;
    };
  }>("/api/dataverse/listSearch", async (req, reply) => {
    try {
      const {
        getDataverseAuth,
        listTables,
        listEntityColumns,
      } = await import("../engine/microsoft/dataverse.js");
      const { resolveCredentialValue, resolveCredentialFields } = await import(
        "../credentials/resolve.js"
      );
      const credentialId = String(req.query.credentialId ?? "");
      const auth = await getDataverseAuth({
        credentialId,
        baseUrl: String(req.query.baseUrl ?? ""),
        getCredential: (k) => process.env[k],
        getCredentialAsync: resolveCredentialValue,
        getCredentialData: resolveCredentialFields,
      });
      const resource = String(req.query.resource ?? "entity");
      const q = String(req.query.q ?? "").toLowerCase();
      if (resource === "column") {
        const entityName = String(req.query.entityName ?? "");
        if (!entityName) {
          return reply.code(400).send({ error: "entityName required" });
        }
        let columns = await listEntityColumns(auth, entityName);
        if (q) {
          columns = columns.filter(
            (c) =>
              c.logicalName.toLowerCase().includes(q) ||
              c.displayName.toLowerCase().includes(q),
          );
        }
        return {
          results: columns.map((c) => ({
            name: `${c.displayName} - (${c.logicalName})`,
            value: c.logicalName,
          })),
        };
      }
      let tables = await listTables(auth);
      if (q) {
        tables = tables.filter(
          (t) =>
            t.logicalName.toLowerCase().includes(q) ||
            t.displayName.toLowerCase().includes(q),
        );
      }
      return {
        results: tables.map((t) => ({
          name: `${t.displayName} - (${t.logicalName})`,
          value: t.logicalName,
        })),
      };
    } catch (err) {
      return reply.code(400).send({ error: errorMessage(err) });
    }
  });

  /** Resource locator search for Microsoft nodes (sites, teams, mail folders). */
  app.get<{
    Querystring: {
      resource?: string;
      q?: string;
      credentialId?: string;
      siteId?: string;
    };
  }>("/api/microsoft/listSearch", async (req, reply) => {
    try {
      const { getGraphToken, graphFetch } = await import(
        "../engine/microsoft/graph.js"
      );
      const { resolveCredentialValue, resolveCredentialFields } = await import(
        "../credentials/resolve.js"
      );
      const credentialId = String(req.query.credentialId ?? "");
      const token = await getGraphToken({
        credentialId,
        getCredential: (k) => process.env[k],
        getCredentialAsync: resolveCredentialValue,
        getCredentialData: resolveCredentialFields,
      });
      if (!token) {
        return reply
          .code(400)
          .send({ error: "Microsoft credential / token required" });
      }
      const resource = String(req.query.resource ?? "site");
      const q = String(req.query.q ?? "*");
      let path = "/sites?search=*";
      if (resource === "site") {
        path = `/sites?search=${encodeURIComponent(q || "*")}`;
      } else if (resource === "mailFolder") {
        path = "/me/mailFolders";
      } else if (resource === "team") {
        path = "/me/joinedTeams";
      } else if (resource === "list" && req.query.siteId) {
        path = `/sites/${req.query.siteId}/lists`;
      }
      const result = await graphFetch(path, token, "GET");
      const body = result.body as { value?: Array<Record<string, unknown>> };
      const results = (body.value ?? []).map((row) => ({
        name: String(row.displayName ?? row.name ?? row.id ?? ""),
        value: String(row.id ?? ""),
        url: row.webUrl,
      }));
      return { results };
    } catch (err) {
      return reply.code(400).send({ error: errorMessage(err) });
    }
  });
}
