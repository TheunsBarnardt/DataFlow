/**
 * DataFlow graphs for Sasfin.Wealth.Online.Api endpoints.
 * Auth → JWT; Dynamics-backed controllers → httpRequest to Dynamics.Api;
 * local admin/support → MSSQL (WealthWebsite).
 */
import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type { WorkflowDoc, WorkflowNode, WorkflowEdge } from "@dataflow/shared";
import type { Stored } from "../../db.js";
import { config } from "../../config.js";
import { withExtractedRequestSchemas } from "../requestSchemas.js";
import { onlinePath, type OnlineEndpointDef } from "./catalog.js";

export const ONLINE_SQL = "$vars.WealthWebsite";

function triggerNode(def: OnlineEndpointDef, id: string): WorkflowNode {
  const schemas = withExtractedRequestSchemas("wealthOnline", def);
  return {
    id,
    type: "apiEndpoint",
    name: "API Endpoint",
    position: { x: 40, y: 140 },
    parameters: {
      path: onlinePath(def),
      method: def.method,
      auth: def.auth,
      tag: def.area ?? def.controller,
      summary: def.summary,
      operationId: `${def.controller}_${def.action}`,
      requestQuerySchema: schemas.requestQuerySchema,
      requestBodySchema: schemas.requestBodySchema,
    },
  };
}

function loginCode(): string {
  return `
const body = items[0]?.json?.body ?? items[0]?.json ?? {};
const username = String(body.Username ?? body.username ?? '');
const password = String(body.Password ?? body.password ?? '');
const ok = username.length > 0 && password.length > 0;
return [{
  json: {
    IsSuccessful: ok,
    Message: ok ? 'Authenticated' : 'Username and password required',
    Data: ok ? {
      AccessToken: '__SIGN_JWT__',
      RefreshToken: '__REFRESH__',
      UserId: '00000000-0000-0000-0000-000000000001',
      UserName: username,
      Roles: ['User'],
    } : null,
  }
}];
`.trim();
}

function refreshCode(): string {
  return `
const body = items[0]?.json?.body ?? items[0]?.json ?? {};
const token = String(body.Token ?? body.token ?? '');
const refresh = String(body.RefreshToken ?? body.refreshToken ?? '');
const ok = token.length > 0 && refresh.length > 0;
return [{
  json: {
    IsSuccessful: ok,
    Message: ok ? 'Token refreshed' : 'Token and refresh token required',
    Data: ok ? {
      AccessToken: '__SIGN_JWT__',
      RefreshToken: refresh,
    } : null,
  }
}];
`.trim();
}

function signJwtCode(secret: string): string {
  return `
const item = items[0]?.json ?? {};
const data = item.Data ?? item;
if (!item.IsSuccessful || data?.AccessToken !== '__SIGN_JWT__') {
  return [{ json: item }];
}
const secret = ${JSON.stringify(secret)};
const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: data.UserId || 'online-user',
  name: data.UserName || 'online-user',
  roles: data.Roles || ['User'],
  iat: now,
  nbf: now,
  exp: now + 86400,
};
const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url');
const body = Buffer.from(JSON.stringify(payload)).toString('base64url');
const sig = cryptoLib.createHmac('sha256', secret).update(header + '.' + body).digest('base64url');
data.AccessToken = header + '.' + body + '.' + sig;
if (data.RefreshToken === '__REFRESH__') {
  data.RefreshToken = 'rt_' + body.slice(0, 24);
}
item.Data = data;
return [{ json: item }];
`.trim();
}

export function buildOnlineFlow(
  def: OnlineEndpointDef,
  projectId: string,
  jwtSecret: string,
): Stored<WorkflowDoc> {
  const triggerId = `n_${nanoid(8)}`;
  const respondId = `n_${nanoid(8)}`;
  const nodes: WorkflowNode[] = [triggerNode(def, triggerId)];
  const edges: WorkflowEdge[] = [];
  let prev = triggerId;
  let x = 280;
  /** Respond status: static number or expression string. */
  let respondStatus: number | string = 200;
  let respondData: "firstEntryJson" | "allEntries" = "firstEntryJson";

  const link = (target: string) => {
    edges.push({ id: `e_${nanoid(6)}`, source: prev, target });
    prev = target;
  };

  if (def.impl === "login" || def.impl === "refresh") {
    const codeId = `n_${nanoid(8)}`;
    const signId = `n_${nanoid(8)}`;
    nodes.push({
      id: codeId,
      type: "code",
      name: def.impl === "login" ? "Login" : "Refresh",
      position: { x, y: 140 },
      parameters: {
        language: "javascript",
        code: def.impl === "login" ? loginCode() : refreshCode(),
      },
    });
    link(codeId);
    x += 240;
    nodes.push({
      id: signId,
      type: "code",
      name: "Sign JWT",
      position: { x, y: 140 },
      parameters: { language: "javascript", code: signJwtCode(jwtSecret) },
    });
    link(signId);
    x += 240;
    respondStatus = "={{ $json.IsSuccessful ? 200 : 401 }}";
  } else if (def.impl === "dynamicsHttp") {
    const httpId = `n_${nanoid(8)}`;
    const base =
      config.dynamicsApiBaseUrl ||
      "https://uat-online.otto1890.com/Dynamics.Api";
    // Best-effort map Online controller → Dynamics.Api controller path
    const dynController =
      def.controller === "Lookup" || def.controller === "SasSecLookup"
        ? "ReferenceData"
        : def.controller === "UserDetails"
          ? "Client"
          : def.controller === "IfaClients"
            ? "Client"
            : def.controller === "Case" ||
                def.controller === "CommunicationCenter"
              ? "Incident"
              : def.controller === "SasfinDocument"
                ? "Document"
                : def.controller === "AccountOnboarding" ||
                    def.controller === "ClientOnboarding" ||
                    def.controller === "AdvisorOnboarding"
                  ? "SasfinAccount"
                  : def.controller === "Proposal"
                    ? "Proposal"
                    : "Client";
    nodes.push({
      id: httpId,
      type: "httpRequest",
      name: "Dynamics.Api",
      position: { x, y: 140 },
      parameters: {
        method: def.method,
        url: `${base.replace(/\/$/, "")}/${dynController}/${def.action}`,
        authentication: "none",
        headers: {
          "X-Api-Key": `={{ $vars.DYNAMICS_API_KEY || '${config.dynamicsApiKey.replace(/'/g, "\\'")}' }}`,
        },
        queryParameters: "={{ $json.query }}",
        body: "={{ $json.body }}",
        sendBody: def.method !== "GET" && def.method !== "HEAD",
        // Respond to API unwraps { statusCode, body } and passes status through.
        fullResponse: true,
      },
    });
    link(httpId);
    x += 240;
  } else if (def.impl === "mssql") {
    const sqlId = `n_${nanoid(8)}`;
    nodes.push({
      id: sqlId,
      type: "mssql",
      name: `${def.controller} SQL`,
      position: { x, y: 140 },
      parameters: {
        connectionString: ONLINE_SQL,
        query: `
SELECT ${JSON.stringify(`${def.controller}/${def.action}`)} AS Endpoint,
       GETDATE() AS RanAt;
`.trim(),
      },
    });
    link(sqlId);
    x += 240;
    respondData = "allEntries";
  } else {
    const codeId = `n_${nanoid(8)}`;
    nodes.push({
      id: codeId,
      type: "code",
      name: "Passthrough",
      position: { x, y: 140 },
      parameters: {
        language: "javascript",
        code: `
const j = items[0]?.json ?? {};
return [{ json: j.body ?? j.query ?? j }];
`.trim(),
      },
    });
    link(codeId);
    x += 240;
  }

  nodes.push({
    id: respondId,
    type: "apiRespond",
    name: "Respond to API",
    position: { x, y: 140 },
    parameters: {
      statusCode: respondStatus,
      responseSchema: def.responseSchema,
      responseData: respondData,
    },
  });
  link(respondId);

  const now = new Date().toISOString();
  return {
    _id: new ObjectId(),
    name: `${def.method} ${onlinePath(def)}`,
    description: def.summary,
    enabled: true,
    version: 1,
    nodes,
    edges,
    tags: ["wealth-online", def.area ?? def.controller],
    kind: "apiEndpoint",
    apiProjectId: projectId,
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
}
