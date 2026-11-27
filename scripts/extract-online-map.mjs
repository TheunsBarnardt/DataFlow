#!/usr/bin/env node
/**
 * Build wealth-online-endpoint-map.json from _wealthOnline-catalog.ts impl tags.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = readFileSync(join(__dirname, "_wealthOnline-catalog.ts"), "utf8");

const entries = [];
// Looser: find each path: + nearby impl:
const pathBlocks = [];
const pathRe = /path:\s*"([^"]+)"/g;
let pm;
while ((pm = pathRe.exec(src))) {
  pathBlocks.push({ path: pm[1], index: pm.index });
}

for (let i = 0; i < pathBlocks.length; i++) {
  const { path, index } = pathBlocks[i];
  const windowStart = Math.max(0, index - 400);
  const windowEnd =
    i + 1 < pathBlocks.length ? pathBlocks[i + 1].index : Math.min(src.length, index + 800);
  const window = src.slice(windowStart, windowEnd);
  const implM = window.match(/impl:\s*"([^"]+)"/);
  const methodM = window.match(/ep\(\s*"[^"]+"\s*,\s*"[^"]+"\s*,\s*"(GET|POST|PUT|DELETE|PATCH)"/);
  const authM = window.match(
    /ep\(\s*"[^"]+"\s*,\s*"[^"]+"\s*,\s*"(?:GET|POST|PUT|DELETE|PATCH)"\s*,\s*"(jwt|none|apiKey|headerBearer)"/,
  );
  const actionM = window.match(/ep\(\s*"([^"]+)"\s*,\s*"([^"]+)"/);
  const impl = implM ? implM[1] : "proxy";
  const method = methodM ? methodM[1] : "GET";
  const auth = authM ? authM[1] : "jwt";
  const controller = actionM ? actionM[1] : "";
  const action = actionM ? actionM[2] : "";

  if (impl === "mssql") {
    entries.push({
      path,
      method,
      auth,
      backend: "mssql",
      credential: "WealthWebsite",
      name: `${controller}.${action}`,
      query: onlineSqlHint(controller, action, path),
    });
  } else if (impl === "login" || impl === "refresh") {
    entries.push({
      path,
      method,
      auth: "none",
      backend: "auth",
      name: `${controller}.${action}`,
    });
  } else {
    // proxy / dynamicsHttp → HTTP to Dynamics.Api
    entries.push({
      path,
      method,
      auth,
      backend: "http",
      name: `${controller}.${action}`,
      url: `={{ ($vars.DYNAMICS_API_BASE || 'http://127.0.0.1:5000') + '/${path}' }}`,
      httpMethod: method,
    });
  }
}

function onlineSqlHint(controller, action, path) {
  const a = String(action || "").toLowerCase();
  if (a.includes("notification") && a.includes("content")) {
    return `
SELECT TOP 1 *
FROM NotificationContent
WHERE CAST(Id AS nvarchar(64)) = {{ "'" + String($json.query.id || '').replace(/'/g,"''") + "'" }};
`.trim();
  }
  if (a.includes("notification")) {
    return `
SELECT TOP 200 *
FROM Notifications
WHERE ({{ !$json.query.Type ? '1=1' : "Type = '" + String($json.query.Type).replace(/'/g,"''") + "'" }})
ORDER BY 1 DESC;
`.trim();
  }
  if (path.includes("PageContent") || controller === "PageContent") {
    return `
SELECT TOP 200 *
FROM PageContent
WHERE ({{ !$json.query.page && !$json.query.Page ? '1=1' : "Page = '" + String($json.query.page || $json.query.Page || '').replace(/'/g,"''") + "'" }})
ORDER BY 1;
`.trim();
  }
  return `-- Online ${controller}.${action}\nSELECT TOP 100 TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE';`;
}

writeFileSync(
  join(__dirname, "wealth-online-endpoint-map.json"),
  JSON.stringify(entries, null, 2),
);
console.log(`Wrote ${entries.length} online map entries`);
console.log(
  "backends:",
  entries.reduce((acc, e) => {
    acc[e.backend] = (acc[e.backend] || 0) + 1;
    return acc;
  }, {}),
);
