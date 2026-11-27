#!/usr/bin/env node
/** Fill remaining wealth unmapped paths with best-effort backend tags. */
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const wealthPath = join(__dirname, "wealth-endpoint-map.json");
const report = JSON.parse(readFileSync(join(__dirname, "reimport-report.json"), "utf8"));
const wealth = JSON.parse(readFileSync(wealthPath, "utf8"));
const byPath = new Set(wealth.map((e) => e.path.toLowerCase()));
const unmapped =
  (report.summary || []).find((s) => s.slug === "wealth")?.unmappedPaths || [];

let added = 0;
for (const path of unmapped) {
  if (byPath.has(path.toLowerCase())) continue;
  const lower = path.toLowerCase();

  if (lower.startsWith("auth/") || lower === "identity") {
    wealth.push({ path, backend: "auth", name: path });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.startsWith("threadmanager/")) {
    wealth.push({
      path,
      backend: "mssql",
      credential: "Wealth_ThreadManager",
      name: path,
      query: `-- ${path}\nSELECT TOP 100 * FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE';`,
    });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.startsWith("document/")) {
    wealth.push({
      path,
      backend: "mssql",
      credential: "Wealth_Documents",
      name: path,
      query: `-- ${path}\nSELECT TOP 50 * FROM DocumentsMetadata;`,
    });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.startsWith("market/") || lower.startsWith("feed/")) {
    wealth.push({
      path,
      backend: "http",
      name: path,
      url: `={{ ($vars.INFRONT_BASE || 'https://mws3.infrontservices.com') + '/' }}`,
      httpMethod: "GET",
    });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.startsWith("portfolio/") || lower.startsWith("crm/")) {
    wealth.push({
      path,
      backend: lower.startsWith("crm/") ? "dynamics" : "mssql",
      credential: lower.startsWith("crm/") ? "dataverse" : "DM_Wealth",
      name: path,
      ...(lower.startsWith("crm/")
        ? {
            operation: "GET",
            type: "ODATA",
            getQuery: "contacts?$top=20",
            entityName: "contact",
          }
        : {
            query: `-- ${path} (DM_Wealth)\nSELECT TOP 1 GETDATE() AS RanAt;`,
          }),
    });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.includes("exclusion") || lower.includes("linkedcrm") || lower.includes("visibleaccount")) {
    wealth.push({
      path,
      backend: "dynamics",
      credential: "dataverse",
      name: path,
      operation: "GET",
      type: "ODATA",
      getQuery: "tct_sasfinaccountses?$top=50",
      entityName: "tct_sasfinaccounts",
    });
    byPath.add(lower);
    added++;
    continue;
  }

  if (lower.startsWith("admin/") || lower.startsWith("manage/")) {
    wealth.push({
      path,
      backend: "mssql",
      credential: "WealthWebsite",
      name: path,
      query: `-- ${path}\nSELECT GETDATE() AS RanAt, N'${path.replace(/'/g, "''")}' AS Endpoint;`,
    });
    byPath.add(lower);
    added++;
  }
}

writeFileSync(wealthPath, JSON.stringify(wealth, null, 2));
console.log(`Added ${added}; wealth map ${wealth.length}`);
