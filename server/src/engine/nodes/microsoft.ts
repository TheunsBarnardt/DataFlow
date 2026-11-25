import sql from "mssql";
import type { ConnectionPool } from "mssql";
import type { NodeHandler } from "../types.js";
import { itemJson, makeItem } from "../items.js";
import { errorMessage } from "../../errorMessage.js";
import {
  excelPath,
  getGraphToken,
  graphFetch,
  graphFetchAll,
  outlookPath,
  sharepointPath,
} from "../microsoft/graph.js";
import {
  createData,
  deleteRecord,
  getData,
  getDataverseAuth,
  modifyEntityLogicalName,
  normalizeOptionSetResponse,
  updateData,
} from "../microsoft/dataverse.js";

/** Compose connection string from SSMS-style credential fields. */
function buildMssqlConnectionStringFromCred(
  cred: Record<string, unknown>,
): string {
  const auth = String(cred.authentication ?? "windows").toLowerCase();
  const plain = String(cred.connectionString ?? "").trim();
  if (auth === "connectionstring") return plain;
  if (plain && !String(cred.server ?? "").trim()) return plain;

  const server = String(cred.server ?? "").trim();
  if (!server) return plain;
  const database = String(cred.database ?? "").trim();
  const encrypt = cred.encrypt !== false && cred.encrypt !== "false";
  const trust =
    cred.trustServerCertificate !== false &&
    cred.trustServerCertificate !== "false";
  const parts: string[] = [`Server=${server}`];
  if (database) parts.push(`Database=${database}`);
  if (auth === "sql" || auth === "sqlserver") {
    const user = String(cred.user ?? cred.username ?? "").trim();
    const password = String(cred.password ?? "");
    if (user) parts.push(`User Id=${user}`);
    if (password) parts.push(`Password=${password}`);
  } else {
    parts.push("Trusted_Connection=True");
  }
  parts.push(`Encrypt=${encrypt ? "True" : "False"}`);
  parts.push(`TrustServerCertificate=${trust ? "True" : "False"}`);

  const protocol = String(cred.networkProtocol ?? "default").toLowerCase();
  if (protocol === "tcp") parts.push("Network Library=dbmssocn");
  else if (protocol === "np") parts.push("Network Library=dbnmpntw");
  else if (protocol === "lpc") parts.push("Network Library=dbmslpcn");

  const packetSize = Number(cred.packetSize ?? 0);
  if (packetSize > 0) parts.push(`Packet Size=${packetSize}`);

  const connectionTimeout = Number(cred.connectionTimeout ?? 30);
  if (Number.isFinite(connectionTimeout)) {
    parts.push(`Connection Timeout=${connectionTimeout}`);
  }

  const requestTimeout = Number(cred.requestTimeout ?? 0);
  if (Number.isFinite(requestTimeout)) {
    parts.push(`Command Timeout=${requestTimeout}`);
  }

  const intent = String(cred.applicationIntent ?? "").trim();
  if (intent && intent !== "ReadWrite") {
    parts.push(`ApplicationIntent=${intent}`);
  }

  if (
    cred.multiSubnetFailover === true ||
    cred.multiSubnetFailover === "true"
  ) {
    parts.push("MultiSubnetFailover=True");
  }

  const extra = String(cred.additionalParameters ?? "").trim();
  if (extra) {
    const cleaned = extra.replace(/^;+|;+$/g, "");
    if (cleaned) parts.push(cleaned);
  }

  return parts.join(";");
}

/** Tedious cannot do Windows/Integrated auth — that needs msnodesqlv8. */
function usesWindowsAuth(connectionString: string): boolean {
  return /(?:^|;)\s*(?:Trusted_Connection|Integrated\s+Security)\s*=\s*(?:true|yes|sspi)\s*(?:;|$)/i.test(
    connectionString,
  );
}

function toOdbcConnectionString(connectionString: string): string {
  let cs = connectionString.trim();
  cs = cs.replace(
    /\b(Trusted_Connection|TrustServerCertificate|Encrypt)\s*=\s*true\b/gi,
    "$1=Yes",
  );
  cs = cs.replace(
    /\b(Trusted_Connection|TrustServerCertificate|Encrypt)\s*=\s*false\b/gi,
    "$1=No",
  );
  if (!/(?:^|;)\s*(?:Server|Data\s+Source|Address|Addr|Network\s+Address)\s*=/i.test(cs)) {
    cs = `Server=localhost;${cs}`;
  }
  if (!/\bDriver\s*=/i.test(cs)) {
    cs = `Driver={ODBC Driver 17 for SQL Server};${cs}`;
  }
  return cs;
}

async function nativeOdbcError(odbc: string): Promise<string> {
  const native = (await import("msnodesqlv8")).default;
  return new Promise((resolve) => {
    native.open(odbc, (err, conn) => {
      if (!err) {
        conn.close(() => undefined);
        resolve("MSSQL Windows auth connection failed (no detail from ODBC driver)");
        return;
      }
      resolve(errorMessage(err));
    });
  });
}

async function openMssqlPool(connectionString: string): Promise<ConnectionPool> {
  if (usesWindowsAuth(connectionString)) {
    const odbc = toOdbcConnectionString(connectionString);
    const sqlV8 = (await import("mssql/msnodesqlv8.js")).default;
    const pool = new sqlV8.ConnectionPool({
      server: "localhost",
      connectionString: odbc,
    } as sql.config);
    try {
      await pool.connect();
      return pool;
    } catch (err) {
      const msg = errorMessage(err);
      if (
        !msg ||
        msg === "Unknown error" ||
        msg === "[object Object]" ||
        /^\{"name":"ConnectionError"\}$/.test(msg)
      ) {
        throw new Error(await nativeOdbcError(odbc));
      }
      throw new Error(msg);
    }
  }
  try {
    const pool = new sql.ConnectionPool(connectionString);
    await pool.connect();
    return pool;
  } catch (err) {
    throw new Error(errorMessage(err));
  }
}

export const mssqlNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  getCredentialData,
  resolve,
}) => {
  const sources = items.length ? items : [makeItem({})];
  const out: ReturnType<typeof makeItem>[] = [];
  const cred = await getCredentialData(
    String(node.parameters.credentialId ?? ""),
  );

  for (const item of sources) {
    const connectionString =
      String(resolve(node.parameters.connectionString ?? "", item) || "") ||
      String(cred.connectionString ?? "") ||
      buildMssqlConnectionStringFromCred(cred) ||
      getCredential("MSSQL_CONNECTION_STRING") ||
      "";
    const query = String(
      resolve(node.parameters.query ?? "SELECT 1 AS ok", item) || "SELECT 1 AS ok",
    );
    if (!connectionString) {
      throw new Error(
        "MSSQL connectionString not set (parameter, credential, or MSSQL_CONNECTION_STRING)",
      );
    }
    const pool = await openMssqlPool(connectionString);
    try {
      const result = await pool.request().query(query);
      const recordset = (result.recordset ?? []) as Record<string, unknown>[];
      if (recordset.length) {
        for (const row of recordset) {
          out.push(makeItem({ ...row }, item));
        }
      } else {
        out.push(makeItem({ rowsAffected: result.rowsAffected }, item));
      }
    } finally {
      await pool.close();
    }
  }

  return { items: out };
};

export const microsoftGraphNode: NodeHandler = async (ctx) => {
  const { node, getCredential, getCredentialAsync, getCredentialData } = ctx;
  const token = await getGraphToken({
    accessToken: String(node.parameters.accessToken ?? ""),
    credentialId: String(node.parameters.credentialId ?? ""),
    getCredential,
    getCredentialAsync,
    getCredentialData,
  });
  const path = String(node.parameters.path ?? "/me");
  const method = String(node.parameters.method ?? "GET");
  if (!token) {
    throw new Error(
      "Microsoft Graph accessToken not set (parameter, credential, or MS_GRAPH_TOKEN)",
    );
  }
  const result = await graphFetch(path, token, method, node.parameters.body);
  const body = result.body as { value?: unknown[] };
  if (Array.isArray(body?.value)) {
    return {
      items: body.value.map((row) =>
        makeItem(row as Record<string, unknown>),
      ),
    };
  }
  return { items: [makeItem(result as unknown as Record<string, unknown>)] };
};

export const outlookNode: NodeHandler = async (ctx) => {
  const token = await getGraphToken({
    accessToken: String(ctx.node.parameters.accessToken ?? ""),
    credentialId: String(ctx.node.parameters.credentialId ?? ""),
    getCredential: ctx.getCredential,
    getCredentialAsync: ctx.getCredentialAsync,
    getCredentialData: ctx.getCredentialData,
  });
  if (!token) {
    throw new Error("Outlook: Microsoft credential / accessToken required");
  }
  const mapped = outlookPath(ctx.node.parameters);
  if (mapped.method === "GET" && !ctx.node.parameters.messageId) {
    const rows = await graphFetchAll(mapped.path, token);
    return {
      items: rows.map((row) => makeItem(row as Record<string, unknown>)),
    };
  }
  const result = await graphFetch(
    mapped.path,
    token,
    mapped.method,
    mapped.body,
  );
  return { items: [makeItem(result as unknown as Record<string, unknown>)] };
};

export const excelOnlineNode: NodeHandler = async (ctx) => {
  const token = await getGraphToken({
    accessToken: String(ctx.node.parameters.accessToken ?? ""),
    credentialId: String(ctx.node.parameters.credentialId ?? ""),
    getCredential: ctx.getCredential,
    getCredentialAsync: ctx.getCredentialAsync,
    getCredentialData: ctx.getCredentialData,
  });
  if (!token) {
    throw new Error("Excel Online: Microsoft credential / accessToken required");
  }
  const mapped = excelPath(ctx.node.parameters);
  const result = await graphFetch(mapped.path, token, mapped.method);
  const body = result.body as { value?: unknown[] };
  if (Array.isArray(body?.value)) {
    return {
      items: body.value.map((row) =>
        makeItem(row as Record<string, unknown>),
      ),
    };
  }
  return { items: [makeItem(result as unknown as Record<string, unknown>)] };
};

export const sharepointNode: NodeHandler = async (ctx) => {
  const token = await getGraphToken({
    accessToken: String(ctx.node.parameters.accessToken ?? ""),
    credentialId: String(ctx.node.parameters.credentialId ?? ""),
    getCredential: ctx.getCredential,
    getCredentialAsync: ctx.getCredentialAsync,
    getCredentialData: ctx.getCredentialData,
  });
  if (!token) {
    throw new Error("SharePoint: Microsoft credential / accessToken required");
  }
  const mapped = sharepointPath(ctx.node.parameters);
  const result = await graphFetch(mapped.path, token, mapped.method);
  const body = result.body as { value?: unknown[] };
  if (Array.isArray(body?.value)) {
    return {
      items: body.value.map((row) =>
        makeItem(row as Record<string, unknown>),
      ),
    };
  }
  return { items: [makeItem(result as unknown as Record<string, unknown>)] };
};

/**
 * Dataverse / Dynamics 365 Web API — parity with
 * TheunsBarnardt/CustomNodes n8n-nodes-dataverse-auth
 * (GET FetchXML|OData, PATCH, POST, OPTIONSET, GLOBALOPTIONSET, ENTITY).
 */
export const dynamicsNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  getCredentialAsync,
  getCredentialData,
  resolve,
}) => {
  const sources = items.length ? items : [makeItem({})];
  const out: ReturnType<typeof makeItem>[] = [];

  const auth = await getDataverseAuth({
    accessToken: String(node.parameters.accessToken ?? ""),
    credentialId: String(node.parameters.credentialId ?? ""),
    baseUrl: String(node.parameters.baseUrl ?? node.parameters.scope ?? ""),
    getCredential,
    getCredentialAsync,
    getCredentialData,
  });

  for (const item of sources) {
    const p = node.parameters;
    let operation = String(
      resolve(p.operation ?? "", item) || "",
    ).toUpperCase();

    // Legacy method/entity/query shape → map onto new ops
    if (!operation) {
      const method = String(
        resolve(p.method ?? "GET", item) || "GET",
      ).toUpperCase();
      if (method === "PATCH") operation = "PATCH";
      else if (method === "POST") operation = "POST";
      else if (method === "DELETE") operation = "DELETE";
      else operation = "GET";
    }

    const entityName = String(
      resolve(
        p.entityName ?? p.entity ?? p.ENTITYNAME ?? "",
        item,
      ) || "",
    );
    const pushResult = (data: unknown) => {
      const body = data as { value?: unknown[] };
      if (Array.isArray(body?.value)) {
        for (const row of body.value) {
          out.push(makeItem(row as Record<string, unknown>, item));
        }
      } else if (data && typeof data === "object") {
        out.push(makeItem(data as Record<string, unknown>, item));
      } else {
        out.push(makeItem({ result: data }, item));
      }
    };

    if (operation === "GET") {
      const type = String(
        resolve(p.type ?? p.queryType ?? "ODATA", item) || "ODATA",
      ).toUpperCase();
      let query = String(
        resolve(p.getQuery ?? p.query ?? "", item) || "",
      );
      // Legacy: entity set + ?$top=… → full OData path segment
      if (type === "ODATA") {
        const looksLikeSet =
          !!entityName &&
          (entityName.endsWith("s") || entityName.includes("("));
        const setName = looksLikeSet
          ? entityName
          : entityName
            ? modifyEntityLogicalName(entityName)
            : "";
        if (setName && !query) {
          query = setName;
        } else if (
          setName &&
          query &&
          (query.startsWith("?") || query.startsWith("$"))
        ) {
          query = `${setName}${query.startsWith("?") ? query : `?${query}`}`;
        }
      }
      const data = await getData(auth, type, entityName, query);
      pushResult(data);
      continue;
    }

    if (operation === "PATCH") {
      const recordId = String(
        resolve(p.recordId ?? p.PATCH_RECORDID ?? "", item) || "",
      );
      if (!entityName || !recordId) {
        throw new Error("Dataverse PATCH requires entityName and recordId");
      }
      const payload = resolve(p.body ?? p.patchData ?? null, item);
      const columns = resolve(p.columns ?? p.column ?? null, item);
      const data = await updateData(
        auth,
        entityName,
        recordId,
        payload,
        columns,
      );
      pushResult(data ?? { ok: true, recordId });
      continue;
    }

    if (operation === "POST") {
      if (!entityName) {
        throw new Error("Dataverse POST requires entityName");
      }
      const payload = resolve(
        p.body ?? p.patchData ?? itemJson(item),
        item,
      );
      const columns = resolve(p.columns ?? p.postcolumn ?? null, item);
      const data = await createData(auth, entityName, payload, columns);
      pushResult(data ?? { ok: true });
      continue;
    }

    if (operation === "OPTIONSET") {
      const osEntity = String(
        resolve(
          p.optionsetEntityName ?? p.OPTIONSET_ENTITYNAME ?? entityName,
          item,
        ) || "",
      );
      const attr = String(
        resolve(
          p.optionsetAttributeName ?? p.OPTIONSET_ATTRIBUTENAME ?? "",
          item,
        ) || "",
      );
      if (!osEntity || !attr) {
        throw new Error(
          "Dataverse OPTIONSET requires optionsetEntityName and optionsetAttributeName",
        );
      }
      const odata = `EntityDefinitions(LogicalName='${osEntity}')/Attributes(LogicalName='${attr}')/Microsoft.Dynamics.CRM.PicklistAttributeMetadata?$select=LogicalName,DisplayName&$expand=OptionSet($select=Options)`;
      const data = await getData(auth, "ODATA", "", odata);
      pushResult(normalizeOptionSetResponse(data));
      continue;
    }

    if (operation === "GLOBALOPTIONSET") {
      const attr = String(
        resolve(
          p.globalAttributeName ?? p.GLOBAL_ATTRIBUTENAME ?? "",
          item,
        ) || "",
      );
      if (!attr) {
        throw new Error(
          "Dataverse GLOBALOPTIONSET requires globalAttributeName",
        );
      }
      const odata = `GlobalOptionSetDefinitions(Name='${attr}')`;
      const data = await getData(auth, "ODATA", "", odata);
      pushResult(normalizeOptionSetResponse(data));
      continue;
    }

    if (operation === "ENTITY") {
      if (!entityName) {
        throw new Error("Dataverse ENTITY requires entityName");
      }
      const idCol = String(
        resolve(p.entityIdColumn ?? p.ENTITY_ID ?? "", item) || "",
      );
      const nameCol = String(
        resolve(p.entityNameColumn ?? p.ENTITY_NAME ?? "", item) || "",
      );
      if (!idCol || !nameCol) {
        throw new Error(
          "Dataverse ENTITY requires entityIdColumn and entityNameColumn",
        );
      }
      const set = modifyEntityLogicalName(entityName);
      const odata = `${set}?$select=${idCol},${nameCol}&$orderby=${nameCol} asc`;
      const data = (await getData(auth, "ODATA", entityName, odata)) as {
        value?: Array<Record<string, unknown>>;
      };
      const options = (data.value ?? []).map((entry) => ({
        Id: entry[idCol],
        Name: entry[nameCol],
      }));
      pushResult({ options });
      continue;
    }

    if (operation === "DELETE") {
      const recordId = String(
        resolve(
          p.recordId ?? p.delete_recordid ?? p.DELETE_RECORDID ?? "",
          item,
        ) || "",
      );
      if (!entityName || !recordId) {
        throw new Error("Dataverse DELETE requires entityName and recordId");
      }
      pushResult(await deleteRecord(auth, entityName, recordId));
      continue;
    }

    throw new Error(`Dataverse: unknown operation ${operation}`);
  }

  return { items: out };
};
