#!/usr/bin/env node
/**
 * Extract wealth-endpoint-map.json from recovered pre-Rust flows.ts switch cases.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = readFileSync(join(__dirname, "_recovered-wealth-flows.ts"), "utf8");

function mapCredential(cs) {
  if (!cs) return "WealthWebsite";
  if (/DM_WEALTH|DM_Wealth/i.test(cs)) return "DM_Wealth";
  if (/DOCS_SQL|Wealth_Documents/i.test(cs)) return "Wealth_Documents";
  if (/MARBLE_MD|MarbleMD/i.test(cs)) return "MarbleMD";
  if (/WEALTH_SQL|WealthWebsite|\$vars\.WealthWebsite/i.test(cs)) return "WealthWebsite";
  if (/NeoX/i.test(cs)) return "NeoX";
  return "WealthWebsite";
}

const switchIdx = src.indexOf("switch (key)");
if (switchIdx < 0) {
  console.error("switch (key) not found");
  process.exit(1);
}
const braceStart = src.indexOf("{", switchIdx);
// Walk braces to find switch end
let depth = 0;
let braceEnd = -1;
for (let i = braceStart; i < src.length; i++) {
  const ch = src[i];
  if (ch === "{") depth++;
  else if (ch === "}") {
    depth--;
    if (depth === 0) {
      braceEnd = i;
      break;
    }
  }
}
const body = src.slice(braceStart + 1, braceEnd);
console.log(`switch body length=${body.length}`);

const caseRe = /case\s+"([^"]+)"\s*:/g;
const casePositions = [];
let m;
while ((m = caseRe.exec(body))) {
  casePositions.push({ path: m[1], index: m.index, end: m.index + m[0].length });
}
console.log(`cases found=${casePositions.length}`);

const entries = [];

for (let i = 0; i < casePositions.length; i++) {
  const cur = casePositions[i];
  // Find the return that applies (may be after fall-through cases)
  let j = i;
  while (j + 1 < casePositions.length) {
    const between = body.slice(casePositions[j].end, casePositions[j + 1].index);
    if (/return\s+(sqlFlow|dynamicsFlow)/.test(between)) break;
    j++;
  }
  const blockEnd =
    j + 1 < casePositions.length ? casePositions[j + 1].index : body.length;
  // Include from first fall-through group start... actually use from last case before return
  let retCase = j;
  while (retCase > i) {
    const between = body.slice(
      casePositions[retCase - 1].end,
      casePositions[retCase].index,
    );
    if (/return\s+(sqlFlow|dynamicsFlow)/.test(between)) {
      retCase = retCase - 1;
      break;
    }
    // keep going — fall through
    const after = body.slice(casePositions[retCase].end, blockEnd);
    if (/return\s+(sqlFlow|dynamicsFlow)/.test(after)) break;
    retCase--;
  }
  const block = body.slice(casePositions[retCase].end, blockEnd);

  if (/return\s+dynamicsFlow/.test(block)) {
    const entity = (block.match(/entity:\s*"([^"]+)"/) || [])[1] || "";
    const query = (block.match(/query:\s*`([^`]*)`/) || [])[1] || "?$top=50";
    const name = (block.match(/name:\s*"([^"]+)"/) || [])[1] || "";
    const getQuery = entity
      ? `${entity}${query.startsWith("?") ? query : query ? `?${query}` : "?$top=50"}`
      : query;
    entries.push({
      path: cur.path,
      backend: "dynamics",
      credential: "dataverse",
      name,
      operation: "GET",
      type: "ODATA",
      getQuery,
      entityName: entity.replace(/es$/, "").replace(/s$/, "") || entity,
    });
    continue;
  }

  if (/return\s+sqlFlow/.test(block)) {
    const sqlName = (block.match(/sqlName:\s*"([^"]+)"/) || [])[1] || "";
    const csMatch = block.match(/connectionString:\s*([A-Z0-9_]+)/);
    const qMatch =
      block.match(/query:\s*`([\s\S]*?)`\.trim\(\)/) ||
      block.match(/query:\s*`([\s\S]*?)`\s*[,}]/);
    const query = qMatch ? qMatch[1].trim() : "SELECT 1 AS ok;";
    entries.push({
      path: cur.path,
      backend: "mssql",
      credential: mapCredential(csMatch ? csMatch[1] : "WEALTH_SQL"),
      name: sqlName,
      query,
    });
  }
}

// Normalize Notifications / Post / Pre
for (const e of entries) {
  if (e.path === "Admin/Notifications") {
    e.query = `
SELECT Id, Description, EditorDescription, MessageType, Priority, IsActive, DateCreated, LastUpdated, LastModifiedBy, LastModifiedDate, Popup
FROM PrePostNotification
ORDER BY LastModifiedDate;
`.trim();
  }
  if (e.path === "Admin/PostNotificationsAsync") {
    e.query = `
SELECT Id, Description, EditorDescription, MessageType, Priority, IsActive, DateCreated, LastUpdated, LastModifiedBy, LastModifiedDate, Popup
FROM PrePostNotification
WHERE IsActive = 1
  AND ({{ !$json.query.messageType ? '1=1' : "MessageType = '" + String($json.query.messageType).replace(/'/g,"''") + "'" }})
ORDER BY LastModifiedDate;
`.trim();
  }
}

const post = entries.find((e) => e.path === "Admin/PostNotificationsAsync");
if (post && !entries.some((e) => e.path === "Admin/PreNotificationsAsync")) {
  entries.push({
    ...JSON.parse(JSON.stringify(post)),
    path: "Admin/PreNotificationsAsync",
    name: "Pre notifications",
  });
}

const seen = new Set();
const unique = [];
for (const e of entries) {
  const k = e.path.toLowerCase();
  if (seen.has(k)) continue;
  seen.add(k);
  unique.push(e);
}

writeFileSync(
  join(__dirname, "wealth-endpoint-map.json"),
  JSON.stringify(unique, null, 2),
);
console.log(`Wrote ${unique.length} wealth map entries`);
console.log(
  "backends:",
  unique.reduce((a, e) => {
    a[e.backend] = (a[e.backend] || 0) + 1;
    return a;
  }, {}),
);
