#!/usr/bin/env node
/**
 * Expand dynamics-fetchxml-map.json to cover every Dynamics.Api controller action.
 * Keeps existing FetchXML / OPTIONSET rows; adds OData GET fallbacks for the rest.
 */
import { readFileSync, writeFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const CONTROLLERS = join(
  "C:/Work/Sasfin.Dynamics.Api/Sasfin.Dynamics.Api/Controllers",
);
const MAP_PATH = join(__dirname, "dynamics-fetchxml-map.json");

function walkControllers(root) {
  const out = [];
  if (!existsSync(root)) return out;
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
  const routeM = before.match(/\[Route\("((?:\\.|[^"\\])*)"\)\]/);
  const route = routeM ? routeM[1] : "[controller]/[action]";
  return { route, controller };
}

function parseControllers(root) {
  const endpoints = [];
  for (const p of walkControllers(root)) {
    const text = readFileSync(p, "utf8");
    const meta = findControllerMeta(text);
    if (!meta) continue;
    const { route, controller } = meta;
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
      let path = route
        .replace("[controller]", controller)
        .replace("[action]", action);
      if (routeSuffix != null && routeSuffix !== "") {
        if (routeSuffix.startsWith("/")) path = path.replace(/\/$/, "") + routeSuffix;
        else path = path.replace(/\/$/, "") + "/" + routeSuffix;
      }
      path = path.replace(/^\/+|\/+$/g, "").replace(/\/+/g, "/");
      endpoints.push({ path, method, controller, action });
    }
  }
  return endpoints;
}

function entitySetGuess(controller) {
  const n = String(controller || "")
    .replace(/Controller$/, "")
    .toLowerCase();
  const known = {
    referencedata: "tct_countries",
    client: "contacts",
    contact: "contacts",
    opportunity: "opportunities",
    systemuser: "systemusers",
    annotation: "annotations",
    sasfinaccount: "tct_sasfinaccountses",
    document: "tct_sasfindocuments",
    bankingdetails: "tct_bankingdetailses",
    connection: "connections",
    lead: "leads",
    incident: "incidents",
    provider: "tct_providers",
    proposal: "sfn_proposals",
    residentcountry: "tct_countries",
    riskassessment: "sfn_riskassessments",
    investmentstrategy: "sfn_investmentstrategies",
    communicationdistribution: "sfn_communicationdistributions",
    lookup: "tct_countries",
    quotation: "sfn_quotations",
  };
  if (known[n]) return known[n];
  if (n.endsWith("y")) return `${n.slice(0, -1)}ies`;
  if (n.endsWith("s")) return n;
  return `${n}s`;
}

function entityLogicalGuess(set) {
  if (set.endsWith("ies")) return set.slice(0, -3) + "y";
  if (set.endsWith("ses") && set.includes("account")) return set.slice(0, -2);
  if (set.endsWith("s")) return set.slice(0, -1);
  return set;
}

const existing = existsSync(MAP_PATH)
  ? JSON.parse(readFileSync(MAP_PATH, "utf8"))
  : [];
const byPath = new Map(existing.map((r) => [String(r.path).toLowerCase(), r]));

const endpoints = parseControllers(CONTROLLERS);
console.log(`Dynamics controller endpoints: ${endpoints.length}`);
console.log(`Existing map entries: ${byPath.size}`);

let added = 0;
for (const ep of endpoints) {
  const key = ep.path.toLowerCase();
  if (byPath.has(key)) continue;
  const set = entitySetGuess(ep.controller);
  const logical = entityLogicalGuess(set);
  let row;
  if (ep.method === "GET") {
    row = {
      path: ep.path,
      operation: "GET",
      type: "ODATA",
      getQuery: `${set}?$top=50`,
      entityName: logical,
    };
  } else if (ep.method === "DELETE") {
    row = {
      path: ep.path,
      operation: "DELETE",
      entityName: logical,
      recordId: "={{ $json.query.id || $json.body?.id || '' }}",
    };
  } else if (ep.method === "PATCH" || ep.method === "PUT") {
    row = {
      path: ep.path,
      operation: "PATCH",
      entityName: logical,
      recordId: "={{ $json.query.id || $json.body?.id || '' }}",
    };
  } else {
    row = {
      path: ep.path,
      operation: "POST",
      entityName: logical,
    };
  }
  existing.push(row);
  byPath.set(key, row);
  added++;
}

writeFileSync(MAP_PATH, JSON.stringify(existing, null, 2));
console.log(`Added ${added}; total map entries ${existing.length}`);
