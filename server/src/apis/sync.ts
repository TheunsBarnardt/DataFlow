import type {
  WorkflowDoc,
  ApiEndpointDoc,
  ApiHttpMethod,
  ApiAuthMode,
  JsonSchema,
} from "@dataflow/shared";
import { ObjectId } from "mongodb";
import { apiEndpoints, type Stored } from "../db.js";

const METHODS = new Set<string>([
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
]);

function methodOf(raw: unknown): ApiHttpMethod {
  const m = String(raw ?? "GET").toUpperCase();
  return (METHODS.has(m) ? m : "GET") as ApiHttpMethod;
}

function authOf(raw: unknown): ApiAuthMode {
  const a = String(raw ?? "none");
  if (a === "jwt" || a === "headerBearer" || a === "apiKey") return a;
  return "none";
}

function schemaOf(raw: unknown): JsonSchema | undefined {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return undefined;
  return raw as JsonSchema;
}

function normalizePath(raw: unknown): string {
  return String(raw ?? "")
    .trim()
    .replace(/^\/+/, "")
    .replace(/\/+$/, "");
}

/** Upsert API endpoint registry rows from canvas apiEndpoint nodes. */
export async function syncApiEndpointsFromWorkflow(
  workflow: Pick<
    WorkflowDoc,
    "_id" | "nodes" | "enabled" | "kind" | "apiProjectId"
  > & { _id: string },
): Promise<ApiEndpointDoc[]> {
  if (workflow.kind !== "apiEndpoint" || !workflow.apiProjectId) {
    // Drop any orphaned registry rows if this workflow was demoted
    await apiEndpoints().deleteMany({ workflowId: String(workflow._id) });
    return [];
  }

  const workflowId = String(workflow._id);
  const apiProjectId = workflow.apiProjectId;
  const endpointNodes = workflow.nodes.filter((n) => n.type === "apiEndpoint");
  const respondNode = workflow.nodes.find((n) => n.type === "apiRespond");
  const keepKeys = new Set<string>();
  const result: ApiEndpointDoc[] = [];

  for (const node of endpointNodes) {
    const path = normalizePath(node.parameters.path);
    if (!path) continue;
    const method = methodOf(node.parameters.method);
    const key = `${method}:${path}`;
    keepKeys.add(key);

    const respondSchema = schemaOf(respondNode?.parameters.responseSchema);
    const legacySchema = schemaOf(node.parameters.responseSchema);
    const rawStatus = respondNode?.parameters.statusCode;
    const numericStatus =
      typeof rawStatus === "number"
        ? rawStatus
        : typeof rawStatus === "string" && /^\d+$/.test(rawStatus.trim())
          ? Number(rawStatus.trim())
          : undefined;
    const legacyStatus = Number(node.parameters.responseStatus ?? NaN);

    const $set = {
      apiProjectId,
      workflowId,
      nodeId: node.id,
      path,
      method,
      summary: String(node.parameters.summary ?? node.name ?? path),
      tag: String(node.parameters.tag ?? path.split("/")[0] ?? "default"),
      auth: authOf(node.parameters.auth),
      operationId: String(
        node.parameters.operationId ??
          `${method}_${path.replace(/[^a-zA-Z0-9]+/g, "_")}`,
      ),
      requestQuerySchema: schemaOf(node.parameters.requestQuerySchema),
      requestBodySchema: schemaOf(node.parameters.requestBodySchema),
      responseSchema: respondSchema ?? legacySchema,
      responseStatus: Number.isFinite(numericStatus)
        ? (numericStatus as number)
        : Number.isFinite(legacyStatus)
          ? legacyStatus
          : 200,
    };

    const existing = await apiEndpoints().findOne({
      apiProjectId,
      method,
      path,
    });

    if (existing) {
      await apiEndpoints().updateOne({ _id: existing._id }, { $set });
      result.push({ ...existing, ...$set, _id: String(existing._id) });
    } else {
      const doc: Stored<ApiEndpointDoc> = {
        _id: new ObjectId(),
        ...$set,
      };
      await apiEndpoints().insertOne(doc);
      result.push({ ...doc, _id: String(doc._id) });
    }
  }

  const orphans = await apiEndpoints().find({ workflowId }).toArray();
  for (const ep of orphans) {
    const key = `${ep.method}:${ep.path}`;
    if (!keepKeys.has(key)) {
      await apiEndpoints().deleteOne({ _id: ep._id });
    }
  }

  return result;
}

export async function deleteApiEndpointsForWorkflow(
  workflowId: string,
): Promise<void> {
  await apiEndpoints().deleteMany({ workflowId });
}

export async function deleteApiEndpointsForProject(
  apiProjectId: string,
): Promise<void> {
  await apiEndpoints().deleteMany({ apiProjectId });
}
