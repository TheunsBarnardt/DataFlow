#!/usr/bin/env node
/**
 * Delete all API projects and reimport from Sasfin C# API sources.
 * - Dynamics: FetchXML / OPTIONSET / OData from dynamics-fetchxml-map.json + dataverseAuth credential
 * - Wealth: MSSQL / Dynamics from wealth-endpoint-map.json + microsoftSql / dataverseAuth credentials
 * - Wealth Online: wealth-online-endpoint-map.json (mssql / http / auth)
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { readdirSync, statSync } from "node:fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const API = process.env.BOTFLOW_API || "http://127.0.0.1:8788";
const MONGO_URI =
  process.env.MONGODB_URI || "mongodb://127.0.0.1:27017/botflow?replicaSet=rs0";
/** Prefer Mongo when API is down: BOTFLOW_MONGO_DIRECT=1 or --mongo */
const USE_MONGO =
  process.env.BOTFLOW_MONGO_DIRECT === "1" ||
  process.argv.includes("--mongo");
const SOURCES = [
  {
    slug: "dynamics",
    name: "Sasfin Dynamics Api",
    description:
      "Flow-backed mirror of Sasfin.Dynamics.Api — CRM Web API with FetchXML / option sets.",
    auth: { mode: "none", schemeName: "ApiKey" },
    controllersRoot: join(
      "C:/Work/Sasfin.Dynamics.Api/Sasfin.Dynamics.Api/Controllers",
    ),
    kind: "dynamics",
  },
  {
    slug: "wealth",
    name: "Sasfin Wealth Api",
    description:
      "Flow-backed mirror of Sasfin.Wealth.Api — JWT auth, MSSQL / Dynamics nodes.",
    auth: {
      mode: "jwt",
      jwtSecret: "botflow-wealth-jwt-dev-secret",
      schemeName: "Bearer",
    },
    controllersRoot: join("C:/Work/Sasfin.Wealth.Api/Wealth.Api/Controllers"),
    kind: "wealth",
  },
  {
    slug: "wealth-online",
    name: "Sasfin Wealth Online Api",
    description:
      "Flow-backed mirror of Sasfin.Wealth.Online.Api — JWT auth, Dynamics.Api + MSSQL.",
    auth: {
      mode: "jwt",
      jwtSecret: "botflow-wealth-online-jwt-dev-secret",
      schemeName: "Bearer",
    },
    controllersRoot: join(
      "C:/Work/Sasfin.Wealth.Online.Api/Sasfin.Wealth.Online.Api/Controllers",
    ),
    kind: "online",
  },
];

function shortId(n = 8) {
  return Math.random().toString(36).slice(2, 2 + n);
}

let mongoDb = null;

async function api(method, path, body) {
  if (USE_MONGO && mongoDb) {
    return mongoApi(method, path, body);
  }
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

async function mongoApi(method, path, body) {
  const { ObjectId } = await import("mongodb");
  const now = new Date().toISOString();
  if (method === "GET" && path === "/api/credentials") {
    const credentials = await mongoDb
      .collection("credentials")
      .find({})
      .project({ name: 1, type: 1 })
      .toArray();
    return {
      credentials: credentials.map((c) => ({
        id: String(c._id),
        _id: String(c._id),
        name: c.name,
        type: c.type,
      })),
    };
  }
  if (method === "GET" && path === "/api/api-projects") {
    const projects = await mongoDb.collection("api_projects").find({}).toArray();
    return {
      projects: projects.map((p) => ({
        ...p,
        _id: String(p._id),
        id: String(p._id),
      })),
    };
  }
  if (method === "DELETE" && path.startsWith("/api/api-projects/")) {
    const id = path.split("/").pop();
    const oidFilters = [{ _id: id }, { apiProjectId: id }];
    if (ObjectId.isValid(id)) {
      try {
        const oid = new ObjectId(id);
        oidFilters.push({ _id: oid }, { apiProjectId: oid });
      } catch {
        /* ignore */
      }
    }
    await mongoDb.collection("workflows").deleteMany({
      $or: oidFilters.filter((f) => "apiProjectId" in f),
    });
    await mongoDb.collection("api_endpoints").deleteMany({
      $or: oidFilters.filter((f) => "apiProjectId" in f),
    });
    await mongoDb.collection("api_projects").deleteMany({
      $or: oidFilters.filter((f) => "_id" in f),
    });
    return { ok: true };
  }
  if (method === "POST" && path === "/api/api-projects") {
    const _id = new ObjectId();
    const doc = {
      ...body,
      _id: String(_id),
      createdBy: "local",
      updatedBy: "local",
      createdAt: now,
      updatedAt: now,
    };
    await mongoDb.collection("api_projects").insertOne(doc);
    return { ...doc, id: String(_id) };
  }
  if (method === "POST" && path === "/api/workflows") {
    const _id = new ObjectId();
    const doc = {
      ...body,
      _id: String(_id),
      createdBy: "local",
      updatedBy: "local",
      createdAt: now,
      updatedAt: now,
    };
    await mongoDb.collection("workflows").insertOne(doc);
    return { ...doc, id: String(_id) };
  }
  throw new Error(`mongoApi unsupported ${method} ${path}`);
}

function walkControllers(root) {
  const out = [];
  if (!existsSync(root)) {
    console.warn(`  ! missing controllers root: ${root}`);
    return out;
  }
  const stack = [root];
  while (stack.length) {
    const dir = stack.pop();
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      const st = statSync(p);
      if (st.isDirectory()) stack.push(p);
      else if (name.endsWith("Controller.cs")) out.push(p);
    }
  }
  return out;
}

function findControllerMeta(text) {
  const ctrlM = text.match(/public\s+(?:partial\s+)?class\s+(\w+)Controller\b/);
  if (!ctrlM) return null;
  const controller = ctrlM[1];
  const before = text.slice(Math.max(0, ctrlM.index - 1200), ctrlM.index);
  const areaM = before.match(/\[Area\(AreaName\.(\w+)\)\]/);
  const area = areaM ? areaM[1] : null;
  const routeM = before.match(/\[Route\("((?:\\.|[^"\\])*)"\)\]/);
  const route = routeM ? routeM[1] : "[controller]/[action]";
  return { area, route, controller };
}

function parseControllers(root) {
  const endpoints = [];
  for (const p of walkControllers(root)) {
    const text = readFileSync(p, "utf8");
    const meta = findControllerMeta(text);
    if (!meta) continue;
    const { area, route, controller } = meta;
    const httpRe =
      /\[Http(Get|Post|Put|Delete|Patch)(?:\("((?:\\.|[^"\\])*)"\))?\]/g;
    const methodRe =
      /public\s+(?:async\s+)?(?:[\w.<>,\[\]\s\?]+?)\s+(\w+)\s*\(([^;{]*)\)\s*(?:\{|;)/;
    let hm;
    while ((hm = httpRe.exec(text))) {
      const method = hm[1].toUpperCase();
      const routeSuffix = hm[2] ?? null;
      const after = text.slice(hm.index + hm[0].length, hm.index + hm[0].length + 900);
      const mm = methodRe.exec(after);
      if (!mm) continue;
      const action = mm[1];
      if (action === controller) continue;
      endpoints.push({
        area,
        controller,
        action,
        method,
        routeTemplate: route,
        routeSuffix,
        params: mm[2].replace(/\s+/g, " ").trim().slice(0, 240),
      });
    }
  }
  return endpoints;
}

function expandRoute(def) {
  let route = def.routeTemplate || "[controller]";
  if (def.area) route = route.replace("[area]", def.area);
  else route = route.replace(/\[area\]\/?/g, "");
  route = route.replace("[controller]", def.controller);
  route = route.replace("[action]", def.action);
  const suffix = def.routeSuffix;
  if (suffix != null && suffix !== "") {
    if (suffix.startsWith("/")) route = route.replace(/\/$/, "") + suffix;
    else route = route.replace(/\/$/, "") + "/" + suffix;
  }
  return route.replace(/^\/+|\/+$/g, "").replace(/\/+/g, "/");
}

function defaultAuth(kind, def) {
  if (kind === "dynamics") return "apiKey";
  const path = expandRoute(def).toLowerCase();
  if (
    path.includes("authenticate") ||
    path.includes("login") ||
    path.includes("token") ||
    path.includes("refresh")
  ) {
    return "none";
  }
  // Anonymous C# [AllowAnonymous] surfaces we know about
  if (
    def.controller === "Admin" &&
    (def.action === "Notifications" ||
      def.action === "PreNotificationsAsync" ||
      def.action === "getWebsiteFaqByCategory" ||
      def.action === "getAnswerByQuestionId")
  ) {
    return "none";
  }
  return "jwt";
}

function loadJsonMap(filename) {
  const p = join(__dirname, filename);
  if (!existsSync(p)) return new Map();
  const arr = JSON.parse(readFileSync(p, "utf8"));
  const map = new Map();
  for (const row of arr) {
    if (row.path) map.set(String(row.path).toLowerCase(), row);
  }
  return map;
}

function modifyEntityLogicalName(name) {
  const n = String(name || "").trim();
  if (!n) return n;
  if (n.endsWith("y")) return `${n.slice(0, -1)}ies`;
  if (n.endsWith("s")) return `${n}es`;
  return `${n}s`;
}

/** Resolve credential ids from BotFlow API once per reimport. */
async function loadCredentialResolver() {
  const listed = await api("GET", "/api/credentials");
  const existing = listed.credentials || listed || [];
  const byName = new Map();
  for (const c of existing) {
    byName.set(String(c.name || "").toLowerCase(), c);
  }
  const dataverse = existing.find(
    (c) =>
      /dataverse|dynamics/i.test(c.type || "") ||
      /dataverse|dynamics|uat/i.test(c.name || ""),
  );
  function resolve(nameOrKind) {
    if (!nameOrKind) return "";
    const key = String(nameOrKind).toLowerCase();
    if (key === "dataverse" || key === "dynamics") {
      return dataverse ? String(dataverse.id || dataverse._id) : "";
    }
    const aliases = {
      wealthwebsite: "WealthWebsite",
      auth: "WealthWebsite",
      dm_wealth: "DM_Wealth",
      dmwealth: "DM_Wealth",
      wealth_documents: "Wealth_Documents",
      docs: "Wealth_Documents",
      marblemd: "MarbleMD",
      neox: "NeoX",
      spyglassdevdb: "SpyGlassDevDB",
      bdadata: "BDAData",
      wealth_threadmanager: "Wealth_ThreadManager",
    };
    const name = aliases[key] || nameOrKind;
    const found =
      byName.get(String(name).toLowerCase()) ||
      existing.find((c) => String(c.name || "").toLowerCase() === String(name).toLowerCase());
    return found ? String(found.id || found._id) : "";
  }
  return { resolve, dataverse, count: existing.length };
}

function buildDynamicsCrmParams(def, fetchMap, creds) {
  const path = expandRoute(def);
  const mapped = fetchMap.get(path.toLowerCase());
  const credentialId = creds.resolve("dataverse");
  const base = {
    credentialId,
    baseUrl: "",
    accessToken: "",
  };

  if (mapped) {
    return {
      ...base,
      operation: mapped.operation || "GET",
      type: mapped.type || "FETCHXML",
      getQuery: mapped.getQuery || "",
      entityName: mapped.entityName || "",
      recordId: mapped.recordId || "",
      optionsetEntityName: mapped.optionsetEntityName || "",
      optionsetAttributeName: mapped.optionsetAttributeName || "",
      globalAttributeName: mapped.globalAttributeName || "",
      entityIdColumn: mapped.entityIdColumn || "",
      entityNameColumn: mapped.entityNameColumn || "",
      columns: mapped.columns || {},
      _mapped: true,
    };
  }

  const entityGuess = String(def.controller || "")
    .replace(/Controller$/, "")
    .toLowerCase();
  const set = modifyEntityLogicalName(entityGuess || "contact");
  return {
    ...base,
    operation:
      def.method === "DELETE"
        ? "DELETE"
        : def.method === "POST" || def.method === "PUT" || def.method === "PATCH"
          ? def.method === "PATCH"
            ? "PATCH"
            : "POST"
          : "GET",
    type: "ODATA",
    getQuery: `${set}?$top=50`,
    entityName: entityGuess,
    recordId: "",
    optionsetEntityName: "",
    optionsetAttributeName: "",
    globalAttributeName: "",
    entityIdColumn: "",
    entityNameColumn: "",
    columns: {},
    _mapped: false,
  };
}

function buildDynamicsFlow(def, projectId, fetchMap, creds) {
  const path = expandRoute(def);
  const triggerId = `n_${shortId()}`;
  const crmId = `n_${shortId()}`;
  const respondId = `n_${shortId()}`;
  const crmParams = buildDynamicsCrmParams(def, fetchMap, creds);
  const { _mapped, ...parameters } = crmParams;
  return {
    name: `${def.method} ${path}`,
    description: `${def.controller}.${def.action}`,
    enabled: true,
    version: 1,
    kind: "apiEndpoint",
    apiProjectId: projectId,
    tags: ["dynamics", def.controller, "reimport", _mapped ? "mapped" : "fallback"],
    nodes: [
      {
        id: triggerId,
        type: "apiEndpoint",
        name: "API Endpoint",
        position: { x: 40, y: 140 },
        parameters: {
          path,
          method: def.method,
          auth: defaultAuth("dynamics", def),
          tag: def.controller,
          summary: `${def.controller}.${def.action}`,
          operationId: `${def.controller}_${def.action}`,
        },
      },
      {
        id: crmId,
        type: "dynamics",
        name: `${def.controller} CRM`,
        position: { x: 320, y: 140 },
        parameters,
      },
      {
        id: respondId,
        type: "apiRespond",
        name: "Respond to API",
        position: { x: 600, y: 140 },
        parameters: {
          statusCode: 200,
          responseData: "firstEntryJson",
          responseSchema: { type: "object", properties: {}, required: [] },
        },
      },
    ],
    edges: [
      { id: `e_${shortId(6)}`, source: triggerId, target: crmId },
      { id: `e_${shortId(6)}`, source: crmId, target: respondId },
    ],
    _stats: {
      mapped: !!_mapped,
      credentialAttached: !!parameters.credentialId,
      stub: false,
    },
  };
}

function lookupWealthMap(def, kind, wealthMap, onlineMap) {
  const path = expandRoute(def);
  const key = path.toLowerCase();
  const shortKey = `${def.controller}/${def.action}`.toLowerCase();
  if (kind === "online") {
    return (
      onlineMap.get(key) ||
      onlineMap.get(shortKey) ||
      [...onlineMap.values()].find(
        (r) =>
          String(r.path || "")
            .toLowerCase()
            .endsWith(`/${def.action}`.toLowerCase()) &&
          String(r.path || "").toLowerCase().includes(String(def.controller).toLowerCase()),
      ) ||
      null
    );
  }
  return wealthMap.get(key) || wealthMap.get(shortKey) || null;
}

function buildWealthishFlow(def, projectId, kind, wealthMap, onlineMap, creds) {
  const path = expandRoute(def);
  const triggerId = `n_${shortId()}`;
  const midId = `n_${shortId()}`;
  const respondId = `n_${shortId()}`;
  const isAuth =
    /authenticate|login|token|refresh/i.test(path) ||
    /authenticate|login|token|refresh/i.test(def.action);

  const mapped = lookupWealthMap(def, kind, wealthMap, onlineMap);
  let mid;
  let stats = { mapped: false, credentialAttached: false, stub: false };

  if (isAuth || (mapped && mapped.backend === "auth")) {
    mid = {
      id: midId,
      type: "code",
      name: "Auth stub",
      position: { x: 320, y: 140 },
      parameters: {
        mode: "runOnceForAllItems",
        code: `return [{ json: { IsSuccessful: true, Message: 'Reimported stub — wire real auth', Access_Token: '', AccessToken: '' } }];`,
      },
    };
    stats = { mapped: !!mapped, credentialAttached: false, stub: true };
  } else if (mapped && mapped.backend === "mssql") {
    const credentialId = creds.resolve(mapped.credential || "WealthWebsite");
    mid = {
      id: midId,
      type: "mssql",
      name: mapped.name || "SQL",
      position: { x: 320, y: 140 },
      parameters: {
        credentialId,
        query: mapped.query || "SELECT 1 AS ok;",
      },
    };
    stats = {
      mapped: true,
      credentialAttached: !!credentialId,
      stub: /SELECT 1 AS ok/i.test(mapped.query || "") || /TODO/i.test(mapped.query || ""),
    };
  } else if (mapped && mapped.backend === "dynamics") {
    const credentialId = creds.resolve("dataverse");
    mid = {
      id: midId,
      type: "dynamics",
      name: mapped.name || `${def.controller} CRM`,
      position: { x: 320, y: 140 },
      parameters: {
        credentialId,
        baseUrl: "",
        accessToken: "",
        operation: mapped.operation || "GET",
        type: mapped.type || "ODATA",
        getQuery: mapped.getQuery || "",
        entityName: mapped.entityName || "",
        recordId: mapped.recordId || "",
        optionsetEntityName: "",
        optionsetAttributeName: "",
        globalAttributeName: "",
        entityIdColumn: "",
        entityNameColumn: "",
        columns: {},
      },
    };
    stats = { mapped: true, credentialAttached: !!credentialId, stub: false };
  } else if (mapped && mapped.backend === "http") {
    mid = {
      id: midId,
      type: "httpRequest",
      name: mapped.name || "HTTP",
      position: { x: 320, y: 140 },
      parameters: {
        url:
          mapped.url ||
          `={{ ($vars.DYNAMICS_API_BASE || 'http://127.0.0.1:5000') + '/${path}' }}`,
        method: mapped.httpMethod || def.method,
        authentication: "none",
        fullResponse: true,
      },
    };
    stats = { mapped: true, credentialAttached: false, stub: false };
  } else if (kind === "online") {
    // Unmapped online → proxy Dynamics.Api (catalog default)
    mid = {
      id: midId,
      type: "httpRequest",
      name: "Dynamics Api",
      position: { x: 320, y: 140 },
      parameters: {
        url: `={{ ($vars.DYNAMICS_API_BASE || 'http://127.0.0.1:5000') + '/${path}' }}`,
        method: def.method,
        authentication: "none",
        fullResponse: true,
      },
    };
    stats = { mapped: false, credentialAttached: false, stub: false };
  } else {
    // Unmapped wealth → visible stub
    const credentialId = creds.resolve("WealthWebsite");
    mid = {
      id: midId,
      type: "mssql",
      name: "SQL",
      position: { x: 320, y: 140 },
      parameters: {
        credentialId,
        query: `-- TODO: map ${def.controller}.${def.action} from source\nSELECT 1 AS ok;`,
      },
    };
    stats = { mapped: false, credentialAttached: !!credentialId, stub: true };
  }

  return {
    name: `${def.method} ${path}`,
    description: `${def.controller}.${def.action}`,
    enabled: true,
    version: 1,
    kind: "apiEndpoint",
    apiProjectId: projectId,
    tags: [
      kind,
      def.controller,
      "reimport",
      stats.mapped ? "mapped" : "unmapped",
      stats.stub ? "stub" : "live",
    ],
    nodes: [
      {
        id: triggerId,
        type: "apiEndpoint",
        name: "API Endpoint",
        position: { x: 40, y: 140 },
        parameters: {
          path,
          method: def.method,
          auth: defaultAuth(kind, def),
          tag: def.area || def.controller,
          summary: `${def.controller}.${def.action}`,
          operationId: `${def.controller}_${def.action}`,
        },
      },
      mid,
      {
        id: respondId,
        type: "apiRespond",
        name: "Respond to API",
        position: { x: 600, y: 140 },
        parameters: {
          statusCode: 200,
          responseData: "firstEntryJson",
          responseSchema: { type: "object", properties: {}, required: [] },
        },
      },
    ],
    edges: [
      { id: `e_${shortId(6)}`, source: triggerId, target: midId },
      { id: `e_${shortId(6)}`, source: midId, target: respondId },
    ],
    _stats: stats,
  };
}

async function deleteAllProjects() {
  const list = await api("GET", "/api/api-projects");
  const projects = list.projects || list || [];
  console.log(`Deleting ${projects.length} API project(s)…`);
  for (const p of projects) {
    const id = p._id || p.id;
    console.log(`  - ${p.slug || p.name} (${id})`);
    await api("DELETE", `/api/api-projects/${id}`);
  }
}

async function importSource(src, maps, creds) {
  console.log(`\nParsing ${src.name} from ${src.controllersRoot}`);
  const endpoints = parseControllers(src.controllersRoot);
  console.log(`  ${endpoints.length} endpoints`);

  // Ensure PreNotificationsAsync exists even if controller parse missed AllowAnonymous twin
  if (src.kind === "wealth") {
    const hasPre = endpoints.some((e) => e.action === "PreNotificationsAsync");
    if (!hasPre) {
      const post = endpoints.find((e) => e.action === "PostNotificationsAsync");
      if (post) {
        endpoints.push({
          ...post,
          action: "PreNotificationsAsync",
          method: "GET",
        });
        console.log("  + injected PreNotificationsAsync");
      }
    }
  }

  const created = await api("POST", "/api/api-projects", {
    name: src.name,
    slug: src.slug,
    description: src.description,
    version: "v1",
    enabled: true,
    auth: src.auth,
    schemas: {
      __seedVersion: { type: "integer", const: 11 },
      __reimportedFrom: { type: "string", const: src.controllersRoot },
    },
  });
  const projectId = created._id || created.id || created.project?._id;
  if (!projectId) {
    throw new Error(`No project id in create response: ${JSON.stringify(created).slice(0, 300)}`);
  }
  console.log(`  project ${projectId}`);

  let withFetch = 0;
  let withOption = 0;
  let mapped = 0;
  let stub = 0;
  let credentialAttached = 0;
  const unmappedPaths = [];

  for (const def of endpoints) {
    const flow =
      src.kind === "dynamics"
        ? buildDynamicsFlow(def, projectId, maps.fetch, creds)
        : buildWealthishFlow(
            def,
            projectId,
            src.kind,
            maps.wealth,
            maps.online,
            creds,
          );
    const st = flow._stats || {};
    delete flow._stats;
    if (st.mapped) mapped++;
    else unmappedPaths.push(expandRoute(def));
    if (st.stub) stub++;
    if (st.credentialAttached) credentialAttached++;

    if (src.kind === "dynamics") {
      const op = flow.nodes[1]?.parameters?.operation;
      const q = flow.nodes[1]?.parameters?.getQuery || "";
      if (op === "GET" && flow.nodes[1]?.parameters?.type === "FETCHXML" && q.includes("<fetch")) {
        withFetch++;
      }
      if (op === "OPTIONSET" || op === "GLOBALOPTIONSET") withOption++;
      // dynamics map hit counts as mapped
      if (flow.tags?.includes("mapped")) {
        /* already counted */
      }
    }
    await api("POST", "/api/workflows", flow);
  }

  console.log(
    `  mapped=${mapped} stub=${stub} credentialAttached=${credentialAttached}` +
      (src.kind === "dynamics"
        ? ` FetchXML=${withFetch} optionsets=${withOption}`
        : ""),
  );
  if (unmappedPaths.length && unmappedPaths.length <= 40) {
    console.log(`  unmapped: ${unmappedPaths.join(", ")}`);
  } else if (unmappedPaths.length) {
    console.log(`  unmapped: ${unmappedPaths.length} paths (see report)`);
  }

  return {
    slug: src.slug,
    endpoints: endpoints.length,
    projectId,
    withFetch,
    withOption,
    mapped,
    stub,
    credentialAttached,
    unmappedPaths,
  };
}

async function main() {
  mkdirSync(join(ROOT, "scripts"), { recursive: true });
  const { spawnSync } = await import("node:child_process");

  if (USE_MONGO) {
    const { MongoClient } = await import("mongodb");
    const client = new MongoClient(MONGO_URI);
    await client.connect();
    mongoDb = client.db();
    console.log(`Mongo direct mode: ${MONGO_URI}`);
    // Keep client open for duration of import — close at end
    globalThis.__botflowMongoClient = client;
  } else {
    // Ensure MSSQL credentials via HTTP when API is up
    console.log("Ensuring API credentials…");
    const ens = spawnSync(process.execPath, [join(__dirname, "ensure-api-credentials.mjs")], {
      cwd: ROOT,
      stdio: "inherit",
      env: process.env,
    });
    if (ens.status !== 0) {
      console.warn("ensure-api-credentials exited", ens.status);
    }
  }

  const fetchMap = loadJsonMap("dynamics-fetchxml-map.json");
  const wealthMap = loadJsonMap("wealth-endpoint-map.json");
  const onlineMap = loadJsonMap("wealth-online-endpoint-map.json");
  console.log(
    `Maps: dynamics=${fetchMap.size} wealth=${wealthMap.size} online=${onlineMap.size}`,
  );
  console.log(USE_MONGO ? `Storage: Mongo` : `API: ${API}`);

  const creds = await loadCredentialResolver();
  console.log(
    `Credentials: ${creds.count}; dataverse=${creds.dataverse ? creds.dataverse.name : "NONE"}; WealthWebsite=${creds.resolve("WealthWebsite") || "NONE"}`,
  );

  await deleteAllProjects();

  const summary = [];
  for (const src of SOURCES) {
    summary.push(
      await importSource(src, { fetch: fetchMap, wealth: wealthMap, online: onlineMap }, creds),
    );
  }

  console.log("\nBackfilling api_endpoints registry…");
  const bf = spawnSync(
    process.execPath,
    [join(__dirname, "backfill-api-endpoints.mjs")],
    {
      cwd: ROOT,
      stdio: "inherit",
      env: { ...process.env, MONGODB_URI: MONGO_URI },
    },
  );
  if (bf.status !== 0) {
    console.warn("backfill exited", bf.status);
  }

  const report = {
    at: new Date().toISOString(),
    api: USE_MONGO ? MONGO_URI : API,
    mode: USE_MONGO ? "mongo" : "http",
    maps: {
      dynamics: fetchMap.size,
      wealth: wealthMap.size,
      online: onlineMap.size,
    },
    credentials: {
      count: creds.count,
      dataverse: creds.dataverse
        ? { id: String(creds.dataverse.id || creds.dataverse._id), name: creds.dataverse.name }
        : null,
      wealthWebsite: creds.resolve("WealthWebsite") || null,
      dmWealth: creds.resolve("DM_Wealth") || null,
    },
    summary,
  };
  writeFileSync(join(__dirname, "reimport-report.json"), JSON.stringify(report, null, 2));
  console.log("\nDone.");
  console.log(JSON.stringify(summary, null, 2));

  if (globalThis.__botflowMongoClient) {
    await globalThis.__botflowMongoClient.close();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
