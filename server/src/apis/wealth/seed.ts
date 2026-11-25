import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type {
  ApiProjectDoc,
  WorkflowDoc,
  WorkflowNode,
  WorkflowEdge,
} from "@dataflow/shared";
import {
  apiProjects,
  workflows,
  type Stored,
} from "../../db.js";
import { config } from "../../config.js";
import { syncApiEndpointsFromWorkflow } from "../sync.js";
import { reapplyEndpointParameters } from "../reapplyParams.js";
import { withExtractedRequestSchemas } from "../requestSchemas.js";
import { WEALTH_ENDPOINTS, wealthPath, type WealthEndpointDef } from "./catalog.js";
import { wealthSchemas } from "./schemas.js";
import { buildRealWealthFlow } from "./flows.js";

const WEALTH_SLUG = "wealth";
/** Bump to force reseed of endpoint flows after catalog/auth/real-flow changes. */
const WEALTH_SEED_VERSION = 5;

function serviceAuthCode(): string {
  return `
const q = items[0]?.json?.query ?? items[0]?.json ?? {};
const clientId = String(q.clientId ?? '');
const clientSecret = String(q.clientSecret ?? '');
const expectedId = ${JSON.stringify(config.wealthClientId)};
const expectedSecret = ${JSON.stringify(config.wealthClientSecret)};
const ok = clientId === expectedId && clientSecret === expectedSecret;
return [{
  json: {
    IsSuccessful: ok,
    Message: ok ? 'Authenticated' : 'Invalid client credentials',
    Access_Token: ok ? '__SIGN_JWT__' : '',
    Roles: ok ? ['Service'] : [],
  }
}];
`.trim();
}

function tokenAuthCode(): string {
  return `
const q = items[0]?.json?.query ?? items[0]?.json ?? {};
const token = String(q.token ?? '');
const ok = token.length > 0;
return [{
  json: {
    IsSuccessful: ok,
    Message: ok ? 'Token exchanged' : 'Token required',
    Access_Token: ok ? '__SIGN_JWT__' : '',
    Roles: ok ? ['User'] : [],
    UserId: ok ? '00000000-0000-0000-0000-000000000001' : '',
    UserName: ok ? 'wealth-user' : '',
  }
}];
`.trim();
}

function signJwtCode(secret: string): string {
  return `
const item = items[0]?.json ?? {};
if (!item.IsSuccessful || item.Access_Token !== '__SIGN_JWT__') {
  return [{ json: item }];
}
const secret = ${JSON.stringify(secret)};
const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: item.UserId || 'service',
  name: item.UserName || 'service',
  roles: item.Roles || [],
  iat: now,
  nbf: now,
  exp: now + 86400,
};
const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url');
const body = Buffer.from(JSON.stringify(payload)).toString('base64url');
const sig = cryptoLib.createHmac('sha256', secret).update(header + '.' + body).digest('base64url');
item.Access_Token = header + '.' + body + '.' + sig;
return [{ json: item }];
`.trim();
}

function buildAuthFlow(
  def: WealthEndpointDef,
  projectId: string,
  jwtSecret: string,
): Stored<WorkflowDoc> {
  const path = wealthPath(def);
  const triggerId = `n_${nanoid(8)}`;
  const codeId = `n_${nanoid(8)}`;
  const signId = `n_${nanoid(8)}`;
  const respondId = `n_${nanoid(8)}`;
  const code =
    def.impl === "serviceAuthenticate" ? serviceAuthCode() : tokenAuthCode();

  const nodes: WorkflowNode[] = [
    {
      id: triggerId,
      type: "apiEndpoint",
      name: "API Endpoint",
      position: { x: 40, y: 120 },
      parameters: (() => {
        const schemas = withExtractedRequestSchemas("wealth", def);
        return {
          path,
          method: def.method,
          auth: def.auth,
          tag: def.controller,
          summary: def.summary,
          operationId: `${def.controller}_${def.action}`,
          requestQuerySchema: schemas.requestQuerySchema,
          requestBodySchema: schemas.requestBodySchema,
        };
      })(),
    },
    {
      id: codeId,
      type: "code",
      name: "Authenticate",
      position: { x: 280, y: 120 },
      parameters: { language: "javascript", code },
    },
    {
      id: signId,
      type: "code",
      name: "Sign JWT",
      position: { x: 520, y: 120 },
      parameters: { language: "javascript", code: signJwtCode(jwtSecret) },
    },
    {
      id: respondId,
      type: "apiRespond",
      name: "Respond to API",
      position: { x: 760, y: 120 },
      parameters: {
        statusCode: 200,
        responseSchema: def.responseSchema,
        responseData: "firstEntryJson",
      },
    },
  ];
  const edges: WorkflowEdge[] = [
    { id: `e_${nanoid(6)}`, source: triggerId, target: codeId },
    { id: `e_${nanoid(6)}`, source: codeId, target: signId },
    { id: `e_${nanoid(6)}`, source: signId, target: respondId },
  ];
  const now = new Date().toISOString();
  return {
    _id: new ObjectId(),
    name: `${def.method} ${path}`,
    description: def.summary,
    enabled: true,
    version: 1,
    nodes,
    edges,
    tags: ["wealth", def.controller],
    kind: "apiEndpoint",
    apiProjectId: projectId,
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
}

/**
 * Ensure the Wealth API project + all mirrored endpoint flows exist.
 * Reseeds when missing, version bumps, or stub/proxy graphs remain.
 */
export async function ensureWealthApiMirror(): Promise<void> {
  const existing = await apiProjects().findOne({ slug: WEALTH_SLUG });
  if (existing) {
    const wfs = await workflows()
      .find({ apiProjectId: String(existing._id), kind: "apiEndpoint" })
      .toArray();
    const seedMeta = (
      existing.schemas as Record<string, { const?: unknown }> | undefined
    )?.__seedVersion;
    const seededVersion = Number(seedMeta?.const ?? seedMeta ?? 0);
    const needsReseed =
      seededVersion !== WEALTH_SEED_VERSION ||
      wfs.length !== WEALTH_ENDPOINTS.length ||
      wfs.some((wf) =>
        wf.nodes.some((n) => {
          if (n.type === "mssql" || n.type === "dynamics") return false;
          if (n.type !== "code") return false;
          const code = String(n.parameters.code ?? "");
          const name = String(n.name ?? "");
          if (name === "Proxy or stub") return true;
          if (code.includes("require(")) return true;
          if (code.includes("Local stub")) return true;
          if (code.includes("_stub")) return true;
          return false;
        }),
      );
    if (!needsReseed) {
      const result = await reapplyEndpointParameters({
        apiProjectId: String(existing._id),
        project: "wealth",
        endpoints: WEALTH_ENDPOINTS,
        pathOf: (d) => wealthPath(d as WealthEndpointDef),
      });
      console.log(
        `Reapplied Wealth API params: ${result.updated}/${result.scanned} workflows`,
      );
      return;
    }
    await workflows().deleteMany({
      apiProjectId: String(existing._id),
      kind: "apiEndpoint",
    });
    const { deleteApiEndpointsForProject } = await import("../sync.js");
    await deleteApiEndpointsForProject(String(existing._id));
    await apiProjects().deleteOne({ _id: existing._id });
    console.log("Reseeding Wealth API mirror with real MSSQL/Dynamics flows…");
  }

  const now = new Date().toISOString();
  const project: Stored<ApiProjectDoc> = {
    _id: new ObjectId(),
    name: "Sasfin Wealth Api",
    slug: WEALTH_SLUG,
    description:
      "Flow-backed mirror of Sasfin.Wealth.Api — JWT auth, Swagger, controller/action routes.",
    version: "v1",
    enabled: true,
    auth: {
      mode: "jwt",
      jwtSecret: config.wealthJwtSecret,
      schemeName: "Bearer",
    },
    schemas: {
      ...wealthSchemas,
      __seedVersion: { type: "integer", const: WEALTH_SEED_VERSION },
    },
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
  await apiProjects().insertOne(project);
  const projectId = String(project._id);

  for (const def of WEALTH_ENDPOINTS) {
    const flow =
      def.impl === "serviceAuthenticate" || def.impl === "tokenAuthenticate"
        ? buildAuthFlow(def, projectId, config.wealthJwtSecret)
        : buildRealWealthFlow(def, projectId);
    await workflows().insertOne(flow);
    await syncApiEndpointsFromWorkflow({
      _id: String(flow._id),
      nodes: flow.nodes,
      enabled: flow.enabled,
      kind: flow.kind,
      apiProjectId: flow.apiProjectId,
    });
  }

  console.log(
    `Seeded Wealth API mirror: /apis/${WEALTH_SLUG} (${WEALTH_ENDPOINTS.length} endpoints)`,
  );
}
