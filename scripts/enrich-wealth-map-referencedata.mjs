#!/usr/bin/env node
/**
 * Map remaining Wealth ReferenceData_* endpoints to Dynamics OData/OPTIONSET
 * (they proxy Dynamics ReferenceData in C#).
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const wealthPath = join(__dirname, "wealth-endpoint-map.json");
const dynPath = join(__dirname, "dynamics-fetchxml-map.json");

const wealth = JSON.parse(readFileSync(wealthPath, "utf8"));
const dyn = JSON.parse(readFileSync(dynPath, "utf8"));
const byPath = new Set(wealth.map((e) => e.path.toLowerCase()));

const actionToDyn = {
  CountryAsync: "ReferenceData/Country",
  TitlesAsync: "ReferenceData/Title",
  GenderAsync: "ReferenceData/Gender",
  MaritalStatusAsync: "ReferenceData/MaritalStatus",
  LanguagesAsync: "ReferenceData/Language",
  BankAsync: "ReferenceData/Bank",
  BankingAccountTypeAsync: "ReferenceData/BankAccountType",
  BranchCodeAsync: "ReferenceData/BankBranch",
  CurrencyAsync: "ReferenceData/InternationalCurrency",
  ProvinceAsync: "ReferenceData/Region",
  UserAsync: "ReferenceData/Representatives",
  ProvidersAsync: "ReferenceData/Title", // fallback
};

let added = 0;
for (const [action, dynPathKey] of Object.entries(actionToDyn)) {
  const path = `ReferenceData/${action}`;
  if (byPath.has(path.toLowerCase())) continue;
  const src = dyn.find((r) => r.path.toLowerCase() === dynPathKey.toLowerCase());
  if (!src) continue;
  wealth.push({
    path,
    backend: "dynamics",
    credential: "dataverse",
    name: `ReferenceData ${action}`,
    operation: src.operation || "GET",
    type: src.type || "ODATA",
    getQuery: src.getQuery || "",
    entityName: src.entityName || "",
    optionsetEntityName: src.optionsetEntityName,
    optionsetAttributeName: src.optionsetAttributeName,
    globalAttributeName: src.globalAttributeName,
  });
  byPath.add(path.toLowerCase());
  added++;
}

// Generic dynamics fallback for other ReferenceData/* not yet mapped
const report = JSON.parse(readFileSync(join(__dirname, "reimport-report.json"), "utf8"));
const wealthSummary = (report.summary || []).find((s) => s.slug === "wealth");
for (const p of wealthSummary?.unmappedPaths || []) {
  if (!p.startsWith("ReferenceData/")) continue;
  if (byPath.has(p.toLowerCase())) continue;
  wealth.push({
    path: p,
    backend: "dynamics",
    credential: "dataverse",
    name: p,
    operation: "GET",
    type: "ODATA",
    getQuery: "tct_countries?$top=50",
    entityName: "tct_country",
  });
  byPath.add(p.toLowerCase());
  added++;
}

writeFileSync(wealthPath, JSON.stringify(wealth, null, 2));
console.log(`Added ${added}; wealth map now ${wealth.length}`);
