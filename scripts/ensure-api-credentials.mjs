#!/usr/bin/env node
/**
 * Upsert microsoftSql credentials used by Wealth / Wealth Online reimport.
 * Reuses existing Dataverse (dataverseAuth) credentials — does not overwrite them.
 *
 * Uses BotFlow HTTP API so data is encrypted with the server SecretBox.
 */
const API = process.env.BOTFLOW_API || "http://127.0.0.1:8788";

/** Catalogs from Sasfin.Wealth.Api appsettings ConnectionStrings (UAT). */
const MSSQL_CREDS = [
  {
    name: "WealthWebsite",
    server: "Jnbut-sql02\\i01",
    database: "WealthWebsite",
  },
  {
    name: "DM_Wealth",
    server: "Jnbut-sql04\\i01",
    database: "DM_Wealth",
  },
  {
    name: "Wealth_Documents",
    server: "Jnbut-sql02\\i01",
    database: "Wealth_Documents",
  },
  {
    name: "MarbleMD",
    server: "Jnbut-sql02\\i01",
    database: "MarbleMD",
  },
  {
    name: "NeoX",
    server: "Jnbut-sql02\\i01",
    database: "NeoX",
  },
  {
    name: "SpyGlassDevDB",
    server: "Jnbut-sql02\\i01",
    database: "SpyGlassDevDB",
  },
  {
    name: "BDAData",
    server: "Jnbut-sql02\\i01",
    database: "BDAData",
  },
  {
    name: "Wealth_ThreadManager",
    server: "Jnbut-sql02\\i01",
    database: "Wealth_ThreadManager",
  },
];

function mssqlData({ server, database }) {
  const connectionString = [
    `Server=${server}`,
    `Database=${database}`,
    "Trusted_Connection=True",
    "TrustServerCertificate=True",
    "Encrypt=False",
  ].join(";");
  return {
    authentication: "windows",
    server,
    database,
    encrypt: false,
    trustServerCertificate: true,
    connectionString,
  };
}

async function api(method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = { raw: text };
  }
  if (!res.ok) {
    throw new Error(`${method} ${path} → ${res.status}: ${text.slice(0, 400)}`);
  }
  return json;
}

async function main() {
  console.log(`API: ${API}`);
  const listed = await api("GET", "/api/credentials");
  const existing = listed.credentials || listed || [];
  console.log(`Existing credentials: ${existing.length}`);

  const byName = new Map(
    existing.map((c) => [String(c.name || "").toLowerCase(), c]),
  );

  const dataverse = existing.find(
    (c) =>
      /dataverse|dynamics/i.test(c.type || "") ||
      /dataverse|dynamics|uat/i.test(c.name || ""),
  );
  console.log(
    "Dataverse:",
    dataverse
      ? { id: dataverse.id || dataverse._id, name: dataverse.name, type: dataverse.type }
      : "(none — create one in Studio for Dynamics nodes)",
  );

  const ensured = [];
  for (const spec of MSSQL_CREDS) {
    const key = spec.name.toLowerCase();
    const found = byName.get(key);
    if (found) {
      console.log(`  keep ${spec.name} (${found.id || found._id})`);
      ensured.push({
        name: spec.name,
        id: String(found.id || found._id),
        type: found.type || "microsoftSql",
        created: false,
      });
      continue;
    }
    const created = await api("POST", "/api/credentials", {
      name: spec.name,
      type: "microsoftSql",
      data: mssqlData(spec),
    });
    const cred = created.credential || created;
    console.log(`  create ${spec.name} (${cred.id || cred._id})`);
    ensured.push({
      name: spec.name,
      id: String(cred.id || cred._id),
      type: "microsoftSql",
      created: true,
    });
  }

  const report = {
    at: new Date().toISOString(),
    api: API,
    dataverse: dataverse
      ? {
          id: String(dataverse.id || dataverse._id),
          name: dataverse.name,
          type: dataverse.type,
        }
      : null,
    mssql: ensured,
  };
  console.log(JSON.stringify(report, null, 2));
  return report;
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
