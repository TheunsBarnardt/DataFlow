/**
 * Build paste-ready Call scopes (METHOD /path) for Roles / Users / API keys.
 * Prefers live BotFlow API endpoints; falls back to C# controller parse + maps.
 */
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");
const API = process.env.BOTFLOW_API || "http://127.0.0.1:8788";

const SOURCES = [
  {
    slug: "dynamics",
    controllersRoot: join(
      "C:/Work/Sasfin.Dynamics.Api/Sasfin.Dynamics.Api/Controllers"
    ),
  },
  {
    slug: "wealth",
    controllersRoot: join("C:/Work/Sasfin.Wealth.Api/Wealth.Api/Controllers"),
  },
  {
    slug: "wealth-online",
    controllersRoot: join(
      "C:/Work/Sasfin.Wealth.Online.Api/Sasfin.Wealth.Online.Api/Controllers"
    ),
  },
];

function walkControllers(dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walkControllers(p, out);
    else if (name.endsWith("Controller.cs")) out.push(p);
  }
  return out;
}

function findControllerMeta(text) {
  const areaM = text.match(/\[Area\("([^"]+)"\)\]/);
  const routeM = text.match(/\[Route\("([^"]+)"\)\]/);
  const classM = text.match(/class\s+(\w+)Controller\b/);
  if (!classM) return null;
  return {
    area: areaM?.[1] || null,
    route: routeM?.[1] || "[controller]",
    controller: classM[1],
  };
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
      const after = text.slice(
        hm.index + hm[0].length,
        hm.index + hm[0].length + 900
      );
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

function scopesFromDefs(defs) {
  const lines = new Set();
  for (const def of defs) {
    const path = expandRoute(def);
    if (!path || path.includes("[area]")) continue;
    lines.add(`${def.method} /${path}`);
  }
  return [...lines].sort((a, b) => a.localeCompare(b));
}

function byPrefix(lines) {
  const map = {};
  for (const line of lines) {
    const path = line.split(/\s+/, 2)[1] || "";
    const seg = path.replace(/^\//, "").split("/")[0] || "(root)";
    (map[seg] ??= []).push(line);
  }
  return map;
}

function wildcards(prefixMap) {
  const out = [];
  for (const seg of Object.keys(prefixMap).sort()) {
    const methods = new Set(prefixMap[seg].map((l) => l.split(/\s+/, 1)[0]));
    if (methods.size === 1) out.push(`${[...methods][0]} /${seg}/*`);
    else out.push(`* /${seg}/*`);
  }
  return out;
}

async function login() {
  const res = await fetch(`${API}/api/v1/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username: "admin", password: "P@ssw0rd" }),
  });
  if (!res.ok) throw new Error(`login ${res.status}`);
  const j = await res.json();
  return j.token;
}

async function liveScopes(token) {
  const headers = { Authorization: `Bearer ${token}` };
  const listRes = await fetch(`${API}/api/api-projects`, { headers });
  if (!listRes.ok) throw new Error(`projects ${listRes.status}`);
  const list = await listRes.json();
  const projects = list.projects || list || [];
  const bySlug = {};
  for (const p of projects) {
    const id = p._id || p.id;
    const slug = p.slug || p.name || id;
    const detailRes = await fetch(`${API}/api/api-projects/${id}`, { headers });
    if (!detailRes.ok) continue;
    const detail = await detailRes.json();
    const endpoints = detail.endpoints || detail.project?.endpoints || [];
    const lines = new Set();
    for (const ep of endpoints) {
      const method = String(ep.method || "GET").toUpperCase();
      let path = String(ep.path || "").trim();
      if (!path) continue;
      if (!path.startsWith("/")) path = "/" + path;
      lines.add(`${method} ${path}`);
    }
    bySlug[slug] = [...lines].sort((a, b) => a.localeCompare(b));
  }
  return bySlug;
}

function scopesFromMap(file, defaultMethod = "GET") {
  if (!existsSync(file)) return [];
  const entries = JSON.parse(readFileSync(file, "utf8"));
  const httpOps = new Set(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]);
  const lines = new Set();
  for (const e of entries) {
    let method = String(e.method || e.httpMethod || "").toUpperCase();
    if (!method || !httpOps.has(method)) {
      const op = String(e.operation || "").toUpperCase();
      method = httpOps.has(op) ? op : defaultMethod;
    }
    let path = String(e.path || "").trim();
    if (!path) continue;
    if (!path.startsWith("/")) path = "/" + path;
    lines.add(`${method} ${path}`);
  }
  return [...lines].sort((a, b) => a.localeCompare(b));
}

function writeBundle(tag, sections) {
  const all = [...new Set(Object.values(sections).flat())].sort((a, b) =>
    a.localeCompare(b)
  );
  writeFileSync(join(ROOT, `scripts/call-scopes-all.txt`), all.join("\n") + "\n");
  writeFileSync(
    join(ROOT, `scripts/call-scopes-wildcards.txt`),
    wildcards(byPrefix(all)).join("\n") + "\n"
  );
  for (const [key, lines] of Object.entries(sections)) {
    writeFileSync(
      join(ROOT, `scripts/call-scopes-${key}.txt`),
      lines.join("\n") + "\n"
    );
    writeFileSync(
      join(ROOT, `scripts/call-scopes-${key}-wildcards.txt`),
      wildcards(byPrefix(lines)).join("\n") + "\n"
    );
  }

  const summary = [];
  summary.push(`Source: ${tag}`);
  summary.push(`Format: METHOD /path  (paste into Call scopes; * wildcards OK)`);
  summary.push("");
  summary.push("Files:");
  summary.push("  scripts/call-scopes-all.txt");
  summary.push("  scripts/call-scopes-wildcards.txt");
  for (const key of Object.keys(sections)) {
    summary.push(`  scripts/call-scopes-${key}.txt`);
    summary.push(`  scripts/call-scopes-${key}-wildcards.txt`);
  }
  summary.push("");
  for (const [key, lines] of Object.entries(sections)) {
    summary.push(`${key}: ${lines.length}`);
    const p = byPrefix(lines);
    for (const seg of Object.keys(p).sort()) {
      summary.push(`  ${seg}: ${p[seg].length}`);
    }
    summary.push("");
  }
  summary.push(`ALL unique: ${all.length}`);
  writeFileSync(join(ROOT, "scripts/call-scopes-README.txt"), summary.join("\n") + "\n");
  console.log(summary.join("\n"));
  return all;
}

async function main() {
  let sections = null;
  let tag = "";

  try {
    const token = await login();
    const live = await liveScopes(token);
    if (Object.keys(live).length) {
      sections = live;
      tag = `live BotFlow API @ ${API}`;
    }
  } catch (e) {
    console.warn("Live API unavailable:", e.message);
  }

  if (!sections) {
    sections = {};
    for (const src of SOURCES) {
      if (existsSync(src.controllersRoot)) {
        sections[src.slug] = scopesFromDefs(parseControllers(src.controllersRoot));
      }
    }
    if (Object.keys(sections).length) {
      tag = "C# controller parse";
    }
  }

  if (!sections || !Object.keys(sections).length) {
    sections = {
      "wealth-online": scopesFromMap(
        join(ROOT, "scripts/wealth-online-endpoint-map.json")
      ),
      wealth: scopesFromMap(join(ROOT, "scripts/wealth-endpoint-map.json")),
      dynamics: scopesFromMap(join(ROOT, "scripts/dynamics-fetchxml-map.json")),
    };
    tag = "endpoint map JSON fallback";
  }

  writeBundle(tag, sections);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
