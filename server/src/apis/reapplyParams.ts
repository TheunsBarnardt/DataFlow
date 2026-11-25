/**
 * Surgically reapply requestQuerySchema / requestBodySchema from extracted
 * ASP.NET schemas onto existing apiEndpoint workflow nodes — without deleting graphs.
 */
import type { JsonSchema, WorkflowDoc, WorkflowNode } from "@dataflow/shared";
import { workflows } from "../db.js";
import { syncApiEndpointsFromWorkflow } from "./sync.js";
import {
  resolveExtractedRequestSchemas,
  withExtractedRequestSchemas,
  type ApiMirrorProject,
} from "./requestSchemas.js";

export type EndpointParamSource = {
  controller: string;
  action: string;
  method?: string;
  requestQuerySchema?: JsonSchema;
  requestBodySchema?: JsonSchema;
};

function operationIdOf(def: EndpointParamSource): string {
  return `${def.controller}_${def.action}`;
}

function schemaEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
}

function patchTriggerNode(
  node: WorkflowNode,
  schemas: {
    requestQuerySchema?: JsonSchema;
    requestBodySchema?: JsonSchema;
  },
): { node: WorkflowNode; changed: boolean } {
  const nextQuery = schemas.requestQuerySchema;
  const nextBody = schemas.requestBodySchema;

  const sameQuery = schemaEqual(node.parameters.requestQuerySchema, nextQuery);
  const sameBody = schemaEqual(node.parameters.requestBodySchema, nextBody);
  // Only clear/set when extract provided a value; leave untouched keys alone if both undefined
  if (
    nextQuery === undefined &&
    nextBody === undefined &&
    sameQuery &&
    sameBody
  ) {
    return { node, changed: false };
  }

  const parameters = { ...node.parameters };
  let changed = false;

  if (nextQuery !== undefined) {
    if (!schemaEqual(parameters.requestQuerySchema, nextQuery)) {
      parameters.requestQuerySchema = nextQuery;
      changed = true;
    }
  }
  if (nextBody !== undefined) {
    if (!schemaEqual(parameters.requestBodySchema, nextBody)) {
      parameters.requestBodySchema = nextBody;
      changed = true;
    }
  } else if (
    parameters.requestBodySchema &&
    typeof parameters.requestBodySchema === "object" &&
    ((parameters.requestBodySchema as { $ref?: string }).$ref ||
      (!(parameters.requestBodySchema as { properties?: unknown }).properties &&
        (parameters.requestBodySchema as { type?: string }).type === "object"))
  ) {
    // Replace opaque object/$ref bodies with nothing when extract has no body —
    // keep as-is to avoid wiping; extract absence means no body params known.
  }

  if (!changed) return { node, changed: false };
  return { changed: true, node: { ...node, parameters } };
}

/**
 * Patch all apiEndpoint workflows for a project from extracted C# request schemas.
 * Matches by operationId (`Controller_Action`), then by path+method if needed.
 */
export async function reapplyEndpointParameters(opts: {
  apiProjectId: string;
  project: ApiMirrorProject;
  endpoints: EndpointParamSource[];
  pathOf?: (def: EndpointParamSource) => string;
}): Promise<{ updated: number; scanned: number }> {
  const { apiProjectId, project, endpoints, pathOf } = opts;
  const enriched = endpoints.map((d) =>
    withExtractedRequestSchemas(project, d),
  );
  const byOp = new Map(enriched.map((d) => [operationIdOf(d), d]));
  const byPathMethod = new Map<string, EndpointParamSource>();
  if (pathOf) {
    for (const d of enriched) {
      const method = String(d.method ?? "GET").toUpperCase();
      byPathMethod.set(`${method}:${pathOf(d)}`, d);
    }
  }

  const wfs = await workflows()
    .find({ apiProjectId, kind: "apiEndpoint" })
    .toArray();

  let updated = 0;
  for (const wf of wfs) {
    let changed = false;
    const nodes = wf.nodes.map((node) => {
      if (node.type !== "apiEndpoint") return node;
      const opId = String(node.parameters.operationId ?? "");
      let def = byOp.get(opId);
      if (!def && pathOf) {
        const method = String(node.parameters.method ?? "GET").toUpperCase();
        const path = String(node.parameters.path ?? "")
          .replace(/^\/+/, "")
          .replace(/\/+$/, "");
        def = byPathMethod.get(`${method}:${path}`);
      }
      if (!def) {
        // Still try extract by parsing operationId
        const parts = opId.split("_");
        if (parts.length >= 2) {
          const action = parts.slice(1).join("_");
          const controller = parts[0]!;
          const extracted = resolveExtractedRequestSchemas(
            project,
            controller,
            action,
          );
          const result = patchTriggerNode(node, extracted);
          if (result.changed) changed = true;
          return result.node;
        }
        return node;
      }
      const result = patchTriggerNode(node, {
        requestQuerySchema: def.requestQuerySchema,
        requestBodySchema: def.requestBodySchema,
      });
      if (result.changed) changed = true;
      return result.node;
    });

    if (!changed) continue;

    await workflows().updateOne(
      { _id: wf._id },
      {
        $set: {
          nodes,
          updatedAt: new Date().toISOString(),
          updatedBy: "system-reapply-params",
        },
      },
    );

    const patched: Pick<
      WorkflowDoc,
      "_id" | "nodes" | "enabled" | "kind" | "apiProjectId"
    > & { _id: string } = {
      _id: String(wf._id),
      nodes,
      enabled: wf.enabled,
      kind: wf.kind,
      apiProjectId: wf.apiProjectId,
    };
    await syncApiEndpointsFromWorkflow(patched);
    updated += 1;
  }

  return { updated, scanned: wfs.length };
}
