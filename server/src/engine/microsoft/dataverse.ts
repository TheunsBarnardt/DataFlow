import type { NodeContext } from "../types.js";

export type DataverseCredSource = {
  accessToken?: string;
  credentialId?: string;
  baseUrl?: string;
  getCredential: NodeContext["getCredential"];
  getCredentialAsync: NodeContext["getCredentialAsync"];
  getCredentialData: NodeContext["getCredentialData"];
};

export type DataverseAuth = {
  token: string;
  /** Org URL without trailing slash, e.g. https://org.crm4.dynamics.com */
  scope: string;
};

/** Match n8n-nodes-dataverse-auth pluralization for entity set names. */
export function modifyEntityLogicalName(entityLogicalName: string): string {
  const name = entityLogicalName.trim();
  if (!name) return name;
  if (name.endsWith("y")) return `${name.slice(0, -1)}ies`;
  if (name.endsWith("s")) return `${name}es`;
  return `${name}s`;
}

function extractEntityNameFromFetchXml(fetchXml: string): string | null {
  const match = fetchXml.match(/<entity\s+name=['"]([^'"]+)['"]/i);
  return match ? match[1] : null;
}

function normalizeScope(raw: string): string {
  return String(raw ?? "")
    .trim()
    .replace(/\/$/, "")
    .replace(/\/api\/data\/v9\.?\d*$/i, "");
}

/** Parse XRM Tooling connection string key=value; pairs. */
export function parseDataverseConnectionString(
  raw: string,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const part of raw.split(";")) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    const key = trimmed.slice(0, eq).trim().toLowerCase().replace(/\s+/g, "");
    const value = trimmed.slice(eq + 1).trim();
    out[key] = value;
  }
  return out;
}

async function fetchClientCredentialsToken(opts: {
  tenant: string;
  clientId: string;
  clientSecret: string;
  scope: string;
}): Promise<string> {
  const body = new URLSearchParams({
    client_id: opts.clientId,
    client_secret: opts.clientSecret,
    scope: `${opts.scope}/.default`,
    grant_type: "client_credentials",
  });
  const res = await fetch(
    `https://login.microsoftonline.com/${opts.tenant}/oauth2/v2.0/token`,
    {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body,
    },
  );
  const json = (await res.json()) as {
    access_token?: string;
    error_description?: string;
  };
  if (!res.ok || !json.access_token) {
    throw new Error(
      json.error_description ||
        `Dataverse OAuth token request failed (${res.status})`,
    );
  }
  return json.access_token;
}

async function fetchRefreshToken(opts: {
  tenant: string;
  clientId: string;
  refreshToken: string;
  scope: string;
  redirectUri?: string;
}): Promise<string> {
  const body = new URLSearchParams({
    client_id: opts.clientId,
    refresh_token: opts.refreshToken,
    grant_type: "refresh_token",
    scope: `${opts.scope}/.default offline_access`,
  });
  if (opts.redirectUri) body.set("redirect_uri", opts.redirectUri);
  const res = await fetch(
    `https://login.microsoftonline.com/${opts.tenant}/oauth2/v2.0/token`,
    {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body,
    },
  );
  const json = (await res.json()) as {
    access_token?: string;
    error_description?: string;
  };
  if (!res.ok || !json.access_token) {
    throw new Error(
      json.error_description ||
        `Dataverse refresh token failed (${res.status})`,
    );
  }
  return json.access_token;
}

/**
 * Resolve Dataverse token + org URL.
 * Supports XrmToolBox-style methods: ClientSecret, ConnectionString, OAuth/MFA, SDK Login, Certificate (token fields).
 */
export async function getDataverseAuth(
  src: DataverseCredSource,
): Promise<DataverseAuth> {
  const inlineToken = String(src.accessToken ?? "").trim();
  let scope = normalizeScope(String(src.baseUrl ?? ""));

  const credentialId = String(src.credentialId ?? "").trim();
  let data: Record<string, unknown> = {};
  if (credentialId) {
    data = await src.getCredentialData(credentialId);
  }

  const method = String(data.authMethod ?? "clientSecret").toLowerCase();

  // Connection string expands into the same shape as other methods
  if (method === "connectionstring" || data.connectionString) {
    const cs = parseDataverseConnectionString(
      String(data.connectionString ?? ""),
    );
    if (cs.url || cs.serviceuri || cs.server) {
      scope =
        scope ||
        normalizeScope(cs.url || cs.serviceuri || cs.server || "");
    }
    if (cs.clientid || cs.appid || cs.applicationid) {
      data.clientId = cs.clientid || cs.appid || cs.applicationid;
    }
    if (cs.clientsecret || cs.secret) {
      data.clientSecret = cs.clientsecret || cs.secret;
    }
    if (cs.username || cs.userid) {
      data.username = cs.username || cs.userid;
    }
    if (cs.password) data.password = cs.password;
    if (cs.redirecturi || cs.replyurl) {
      data.redirectUri = cs.redirecturi || cs.replyurl;
    }
    if (cs.authtype || cs.authenticationtype) {
      data._csAuthType = (cs.authtype || cs.authenticationtype).toLowerCase();
    }
    if (cs.clientid && !data.oauthClientId) {
      data.oauthClientId = cs.clientid;
    }
  }

  if (!scope) {
    scope = normalizeScope(
      String(
        data.scope ??
          data.baseUrl ??
          data.resource ??
          data.environmentUrl ??
          "",
      ),
    );
  }

  if (inlineToken && scope) {
    return { token: inlineToken, scope };
  }

  const existing = String(
    data.accessToken ?? data.token ?? data.bearerToken ?? "",
  ).trim();
  if (existing && scope) {
    return { token: existing, scope };
  }

  const csAuth = String(data._csAuthType ?? "");
  const isClientSecret =
    method === "clientsecret" ||
    csAuth === "clientsecret" ||
    (method === "connectionstring" &&
      (csAuth === "clientsecret" || Boolean(data.clientSecret)));

  const isOAuth =
    method === "oauthmfa" ||
    method === "sdklogin" ||
    csAuth === "oauth" ||
    (method === "connectionstring" && csAuth === "oauth");

  const isCert =
    method === "certificate" ||
    csAuth === "certificate";

  if (isClientSecret && scope) {
    const tenant = String(data.tenantId ?? data.tenant ?? "common");
    const clientId = String(data.clientId ?? data.appId ?? "");
    const clientSecret = String(data.clientSecret ?? data.secret ?? "");
    if (clientId && clientSecret) {
      const token = await fetchClientCredentialsToken({
        tenant,
        clientId,
        clientSecret,
        scope,
      });
      return { token, scope };
    }
  }

  if (isOAuth && scope) {
    const tenant = String(
      data.tenantIdOAuth ?? data.tenantId ?? data.tenant ?? "common",
    );
    const clientId = String(
      data.oauthClientId ?? data.clientId ?? data.appId ?? "",
    );
    const refresh = String(data.refreshToken ?? "").trim();
    if (clientId && refresh) {
      const token = await fetchRefreshToken({
        tenant,
        clientId,
        refreshToken: refresh,
        scope,
        redirectUri: String(data.redirectUri ?? "") || undefined,
      });
      return { token, scope };
    }
    throw new Error(
      "Dataverse OAuth/MFA: set Access token or Refresh token (after interactive Microsoft login). Interactive browser login is not run inside the workflow engine.",
    );
  }

  if (isCert) {
    throw new Error(
      "Dataverse Certificate auth: paste an Access token obtained with your cert-backed app, or use Client Id/Secret. Full client-assertion PEM exchange is not enabled in this runtime yet.",
    );
  }

  // Legacy: client id/secret without authMethod
  const tenant = String(data.tenantId ?? data.tenant ?? "common");
  const clientId = String(data.clientId ?? data.appId ?? "");
  const clientSecret = String(data.clientSecret ?? data.secret ?? "");
  if (clientId && clientSecret && scope) {
    const token = await fetchClientCredentialsToken({
      tenant,
      clientId,
      clientSecret,
      scope,
    });
    return { token, scope };
  }

  const envToken =
    (await src.getCredentialAsync("DYNAMICS_TOKEN")) ||
    src.getCredential("DYNAMICS_TOKEN") ||
    (await src.getCredentialAsync("MS_GRAPH_TOKEN")) ||
    src.getCredential("MS_GRAPH_TOKEN") ||
    "";
  if (!scope) {
    scope = normalizeScope(
      src.getCredential("DYNAMICS_BASE_URL") ||
        (await src.getCredentialAsync("DYNAMICS_BASE_URL")) ||
        "",
    );
  }
  if (envToken && scope) {
    return { token: envToken, scope };
  }

  throw new Error(
    "Dataverse: choose a connection method (Client Id/Secret, Connection String, OAuth/MFA, …) and fill required fields",
  );
}

const ODATA_HEADERS: Record<string, string> = {
  "OData-MaxVersion": "4.0",
  "OData-Version": "4.0",
  "Content-Type": "application/json; charset=utf-8",
  Accept: "application/json",
  Prefer: 'odata.include-annotations="*"',
};

export async function dataverseFetch(
  auth: DataverseAuth,
  apiPath: string,
  method = "GET",
  body?: unknown,
): Promise<{ statusCode: number; body: unknown }> {
  const path = apiPath.startsWith("/") ? apiPath : `/${apiPath}`;
  const url = path.startsWith("http")
    ? path
    : `${auth.scope}${path.startsWith("/api/") ? path : `/api/data/v9.2/${path.replace(/^\//, "")}`}`;

  const res = await fetch(url, {
    method,
    headers: {
      ...ODATA_HEADERS,
      authorization: `Bearer ${auth.token}`,
      "OData-MaxPageSize": "5000",
    },
    body:
      body === undefined || method === "GET" || method === "DELETE"
        ? undefined
        : typeof body === "string"
          ? body
          : JSON.stringify(body),
  });
  const text = await res.text();
  let data: unknown = text;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    /* keep text */
  }
  if (!res.ok) {
    throw new Error(
      `Dataverse API error: ${res.status} - ${res.statusText}. Details: ${typeof data === "string" ? data : JSON.stringify(data)}`,
    );
  }
  return { statusCode: res.status, body: data };
}

/** Merge JSON body + column map / columnValues into one payload. */
export function mergeColumnPayload(
  payload: unknown,
  columns?: unknown,
): Record<string, unknown> {
  const body: Record<string, unknown> =
    payload && typeof payload === "object" && !Array.isArray(payload)
      ? { ...(payload as Record<string, unknown>) }
      : {};

  if (!columns) return body;

  if (
    typeof columns === "object" &&
    columns !== null &&
    "columnValues" in columns &&
    Array.isArray((columns as { columnValues: unknown }).columnValues)
  ) {
    for (const row of (columns as { columnValues: Array<Record<string, unknown>> })
      .columnValues) {
      const name = String(row.columnName ?? row.key ?? "").trim();
      if (name) body[name] = row.columnValue ?? row.value;
    }
    return body;
  }

  if (typeof columns === "object" && columns !== null && !Array.isArray(columns)) {
    for (const [k, v] of Object.entries(columns as Record<string, unknown>)) {
      if (k) body[k] = v;
    }
  }
  return body;
}

export async function getData(
  auth: DataverseAuth,
  type: string,
  entityName: string,
  query: string,
): Promise<unknown> {
  let fullApiUrl = "";
  const t = type.toUpperCase();

  if (t === "FETCHXML") {
    const entityLogicalName = extractEntityNameFromFetchXml(query);
    if (!entityLogicalName) {
      throw new Error("Failed to extract entity name from FetchXML.");
    }
    const set = modifyEntityLogicalName(entityLogicalName);
    fullApiUrl = `/api/data/v9.2/${set}?fetchXml=${encodeURIComponent(query)}`;
  } else if (t === "ODATA") {
    fullApiUrl = `/api/data/v9.2/${query.replace(/^\//, "")}`;
  } else if (t === "COLUMN") {
    const set = modifyEntityLogicalName(entityName);
    fullApiUrl = `/api/data/v9.2/${set}?$select=${query}`;
  } else {
    throw new Error(`Unknown Dataverse GET type: ${type}`);
  }

  const result = await dataverseFetch(auth, fullApiUrl, "GET");
  return result.body;
}

export async function createData(
  auth: DataverseAuth,
  entityName: string,
  payload: unknown,
  columns?: unknown,
): Promise<unknown> {
  const body = mergeColumnPayload(payload, columns);
  const set = modifyEntityLogicalName(entityName);
  const result = await dataverseFetch(
    auth,
    `/api/data/v9.2/${set}`,
    "POST",
    body,
  );
  return result.body ?? { statusCode: result.statusCode, ok: true };
}

export async function updateData(
  auth: DataverseAuth,
  entityName: string,
  recordId: string,
  payload: unknown,
  columns?: unknown,
): Promise<unknown> {
  const body = mergeColumnPayload(payload, columns);
  const set = modifyEntityLogicalName(entityName);
  const result = await dataverseFetch(
    auth,
    `/api/data/v9.2/${set}(${recordId})`,
    "PATCH",
    body,
  );
  return result.body ?? { statusCode: result.statusCode, ok: true };
}

/** DELETE by id — parity with CustomNodes PR #4 DeleteRecord. */
export async function deleteRecord(
  auth: DataverseAuth,
  entityName: string,
  recordId: string,
): Promise<{ success: true; message: string }> {
  const set = modifyEntityLogicalName(entityName);
  await dataverseFetch(auth, `/api/data/v9.2/${set}(${recordId})`, "DELETE");
  return {
    success: true,
    message: `Record ${recordId} deleted successfully from ${entityName}`,
  };
}

export async function listTables(
  auth: DataverseAuth,
): Promise<{ logicalName: string; displayName: string }[]> {
  const result = await dataverseFetch(
    auth,
    `/api/data/v9.2/EntityDefinitions?$select=LogicalName,DisplayName`,
    "GET",
  );
  const value = (result.body as { value?: Array<Record<string, unknown>> })
    ?.value;
  return (value ?? []).map((entity) => ({
    logicalName: String(entity.LogicalName ?? ""),
    displayName: String(
      (entity.DisplayName as { UserLocalizedLabel?: { Label?: string } })
        ?.UserLocalizedLabel?.Label ?? entity.LogicalName ?? "",
    ),
  }));
}

export async function listEntityColumns(
  auth: DataverseAuth,
  entityName: string,
): Promise<{ logicalName: string; displayName: string }[]> {
  const result = await dataverseFetch(
    auth,
    `/api/data/v9.2/EntityDefinitions(LogicalName='${entityName}')/Attributes?$select=LogicalName,DisplayName`,
    "GET",
  );
  const value = (result.body as { value?: Array<Record<string, unknown>> })
    ?.value;
  return (value ?? []).map((attribute) => ({
    logicalName: String(attribute.LogicalName ?? ""),
    displayName: String(
      (attribute.DisplayName as { UserLocalizedLabel?: { Label?: string } })
        ?.UserLocalizedLabel?.Label ?? attribute.LogicalName ?? "",
    ),
  }));
}

function mapOptionSetOptions(options: unknown[]): { Id: unknown; Name: string }[] {
  return options.map((entry) => {
    const e = entry as {
      Value?: unknown;
      Label?: { LocalizedLabels?: Array<{ Label?: string }> };
    };
    return {
      Id: e.Value,
      Name: String(e.Label?.LocalizedLabels?.[0]?.Label ?? e.Value ?? ""),
    };
  });
}

/** Normalize OptionSet / GlobalOptionSet API shapes into { options: [{Id,Name}] }. */
export function normalizeOptionSetResponse(data: unknown): Record<string, unknown> {
  const d = data as {
    Options?: unknown[];
    OptionSet?: { Options?: unknown[] };
    value?: Array<{ OptionSet?: { Options?: unknown[] } }>;
  };

  if (Array.isArray(d.Options)) {
    return { options: mapOptionSetOptions(d.Options) };
  }
  if (d.OptionSet?.Options) {
    return { options: mapOptionSetOptions(d.OptionSet.Options) };
  }
  if (d.value?.[0]?.OptionSet?.Options) {
    return { options: mapOptionSetOptions(d.value[0].OptionSet.Options) };
  }
  return (data as Record<string, unknown>) ?? {};
}
