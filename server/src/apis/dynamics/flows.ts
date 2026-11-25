/**
 * DataFlow graphs for Sasfin.Dynamics.Api endpoints.
 * Controllers call Dynamics 365 via the CRM Web API — mirrored with `dynamics` nodes.
 */
import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type { WorkflowDoc, WorkflowNode, WorkflowEdge } from "@dataflow/shared";
import type { Stored } from "../../db.js";
import { withExtractedRequestSchemas } from "../requestSchemas.js";
import { dynamicsPath, type DynamicsEndpointDef } from "./catalog.js";

/** Matches Dynamics.Api ConnectionStrings:Azure url host. */
export const DYNAMICS_BASE = "https://sasfinuat.crm4.dynamics.com";

function triggerNode(def: DynamicsEndpointDef, id: string): WorkflowNode {
  const schemas = withExtractedRequestSchemas("dynamics", def);
  return {
    id,
    type: "apiEndpoint",
    name: "API Endpoint",
    position: { x: 40, y: 140 },
    parameters: {
      path: dynamicsPath(def),
      method: def.method,
      auth: def.auth,
      tag: def.controller,
      summary: def.summary,
      operationId: `${def.controller}_${def.action}`,
      requestQuerySchema: schemas.requestQuerySchema,
      requestBodySchema: schemas.requestBodySchema,
    },
  };
}

function crmMethod(http: string): string {
  if (http === "DELETE") return "DELETE";
  if (http === "POST" || http === "PUT" || http === "PATCH") return "POST";
  return "GET";
}

function odataQuery(def: DynamicsEndpointDef): string {
  const entity = def.entity ?? "contacts";
  // Prefer id-style lookups when a guid query param is present
  const props = (def.requestQuerySchema?.properties ?? {}) as Record<
    string,
    unknown
  >;
  const keys = Object.keys(props);
  const idKey =
    keys.find((k) => /^(id|.*Id)$/i.test(k)) ??
    keys.find((k) => /Id$/i.test(k));
  if (def.method === "GET" && idKey) {
    return `?$filter=${entity === "contacts" ? "contactid" : `${entity.slice(0, -1)}id`} eq {{ $json.query.${idKey} }}&$top=50`;
  }
  return "?$top=50";
}

export function buildDynamicsFlow(
  def: DynamicsEndpointDef,
  projectId: string,
): Stored<WorkflowDoc> {
  const triggerId = `n_${nanoid(8)}`;
  const dynId = `n_${nanoid(8)}`;
  const respondId = `n_${nanoid(8)}`;
  const entity = def.entity ?? "contacts";

  const nodes: WorkflowNode[] = [
    triggerNode(def, triggerId),
    {
      id: dynId,
      type: "dynamics",
      name: `${def.controller} CRM`,
      position: { x: 320, y: 140 },
      parameters: {
        baseUrl: DYNAMICS_BASE,
        entity,
        method: crmMethod(def.method),
        query: odataQuery(def),
        accessToken: "={{ $vars.DYNAMICS_TOKEN || '' }}",
        body:
          def.method === "GET" || def.method === "DELETE"
            ? undefined
            : "={{ $json.body }}",
      },
    },
    {
      id: respondId,
      type: "apiRespond",
      name: "Respond to API",
      position: { x: 600, y: 140 },
      parameters: {
        statusCode: 200,
        responseSchema: def.responseSchema,
        responseData: "firstEntryJson",
      },
    },
  ];
  const edges: WorkflowEdge[] = [
    { id: `e_${nanoid(6)}`, source: triggerId, target: dynId },
    { id: `e_${nanoid(6)}`, source: dynId, target: respondId },
  ];
  const now = new Date().toISOString();
  return {
    _id: new ObjectId(),
    name: `${def.method} ${dynamicsPath(def)}`,
    description: def.summary,
    enabled: true,
    version: 1,
    nodes,
    edges,
    tags: ["dynamics", def.controller],
    kind: "apiEndpoint",
    apiProjectId: projectId,
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
}
