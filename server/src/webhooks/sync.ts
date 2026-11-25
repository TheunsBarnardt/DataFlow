import { nanoid } from "nanoid";
import type { WorkflowDoc, WebhookDoc, NodeType } from "@dataflow/shared";
import { webhooks, type Stored } from "../db.js";
import { ObjectId } from "mongodb";

const METHODS = new Set([
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
]);

const HOOK_TRIGGER_KINDS: Partial<
  Record<NodeType, NonNullable<WebhookDoc["kind"]>>
> = {
  webhook: "webhook",
  formTrigger: "form",
  sseTrigger: "sse",
  outlookTrigger: "outlook",
  teamsTrigger: "teams",
  whatsappTrigger: "whatsapp",
};

function methodOf(raw: unknown): WebhookDoc["method"] {
  const m = String(raw ?? "POST").toUpperCase();
  return (METHODS.has(m) ? m : "POST") as WebhookDoc["method"];
}

function pathForNode(
  node: WorkflowDoc["nodes"][0],
  kind: NonNullable<WebhookDoc["kind"]>,
): string {
  let path = String(node.parameters.path ?? "").trim();
  if (!path) {
    if (kind === "form") {
      path = `form-${String(node.parameters.formId ?? node.id).replace(/\W+/g, "-")}`;
    } else if (kind === "sse") {
      path = `sse-${node.id}`;
    } else if (kind !== "webhook") {
      path = `${kind}-${node.id}`;
    } else {
      path = nanoid(12);
    }
    node.parameters.path = path;
  }
  return path;
}

/** Upsert production webhook registry rows from canvas webhook + inbound trigger nodes. */
export async function syncWebhooksFromWorkflow(
  workflow: Pick<WorkflowDoc, "_id" | "nodes" | "enabled"> & { _id: string },
): Promise<WebhookDoc[]> {
  const workflowId = String(workflow._id);
  const hookNodes = workflow.nodes.filter((n) => n.type in HOOK_TRIGGER_KINDS);
  const keepKeys = new Set<string>();
  const result: WebhookDoc[] = [];

  for (const node of hookNodes) {
    const kind = HOOK_TRIGGER_KINDS[node.type]!;
    const path = pathForNode(node, kind);
    const method = methodOf(node.parameters.method ?? "POST");
    const authentication: WebhookDoc["authentication"] =
      node.parameters.authentication === "headerBearer" ||
      node.parameters.authentication === "basic"
        ? "headerBearer"
        : "none";
    let secret = String(node.parameters.secret ?? "").trim();
    if (authentication === "headerBearer" && !secret) {
      secret = nanoid(24);
      node.parameters.secret = secret;
    }
    if (!secret) secret = nanoid(24);

    const key = `${kind}:${path}`;
    keepKeys.add(key);
    const existing = await webhooks().findOne({
      workflowId,
      path,
      kind,
    });

    if (existing) {
      const $set = {
        nodeId: node.id,
        method,
        secret,
        authentication,
        kind,
      };
      await webhooks().updateOne({ _id: existing._id }, { $set });
      result.push({ ...existing, ...$set, _id: String(existing._id) });
    } else {
      // Path is globally unique — reuse by path if present
      const byPath = await webhooks().findOne({ path });
      if (byPath) {
        const $set = {
          workflowId,
          nodeId: node.id,
          method,
          secret,
          authentication,
          kind,
        };
        await webhooks().updateOne({ _id: byPath._id }, { $set });
        result.push({ ...byPath, ...$set, _id: String(byPath._id) });
      } else {
        const doc: Stored<WebhookDoc> = {
          _id: new ObjectId(),
          workflowId,
          nodeId: node.id,
          path,
          secret,
          method,
          authentication,
          kind,
        };
        await webhooks().insertOne(doc);
        result.push({
          ...doc,
          _id: String(doc._id),
        });
      }
    }
  }

  // Drop orphaned hooks for this workflow that no longer match a synced node
  const orphans = await webhooks().find({ workflowId }).toArray();
  for (const hook of orphans) {
    const kind = hook.kind ?? "webhook";
    if (!Object.values(HOOK_TRIGGER_KINDS).includes(kind)) continue;
    const key = `${kind}:${hook.path}`;
    if (!keepKeys.has(key)) {
      await webhooks().deleteOne({ _id: hook._id });
    }
  }

  return result;
}
