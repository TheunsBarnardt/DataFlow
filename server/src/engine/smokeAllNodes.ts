/**
 * One in-process smoke case per registered node type (no Mongo / external APIs).
 * Run: npx tsx src/engine/smokeAllNodes.ts  (from server workspace)
 */
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import type { NodeType, WorkflowNode } from "@dataflow/shared";
import { DAY_ONE_NODE_TYPES } from "@dataflow/shared";
import { registry } from "./registry.js";
import { normalizeItems, itemJson, type WorkflowItem } from "./items.js";
import {
  resolveExpressionsInValue,
  type ExpressionRunContext,
  type RunStoreEntry,
} from "./resolveExpressions.js";
import type { NodeContext, NodeResult } from "./types.js";

function ctx(
  type: NodeType,
  parameters: Record<string, unknown>,
  items: WorkflowItem[],
  opts?: {
    inputsByHandle?: Record<string, WorkflowItem[]>;
    runInput?: unknown;
    runStore?: Map<string, RunStoreEntry>;
  },
): NodeContext {
  const vars: Record<string, string> = { ENV: "test" };
  const store = opts?.runStore ?? new Map();
  const nodeCtx: Record<string, unknown> = {};
  const expressionCtx: ExpressionRunContext = {
    vars,
    runStore: store,
    parameters,
    throwOnError: true,
  };
  const resolve = <T>(value: T, item?: WorkflowItem): T =>
    resolveExpressionsInValue(value, vars, item, items, {
      ...expressionCtx,
      itemIndex: item ? Math.max(0, items.indexOf(item)) : 0,
    });
  return {
    node: {
      id: `n-${type}`,
      type,
      name: type,
      position: { x: 0, y: 0 },
      parameters: resolveExpressionsInValue(parameters, vars, undefined, undefined, {
        vars,
        runStore: store,
        throwOnError: false,
      }) as Record<string, unknown>,
    } as WorkflowNode,
    items,
    inputsByHandle: opts?.inputsByHandle ?? { main: items, "0": items },
    runInput: opts?.runInput,
    getCredential: () => undefined,
    getCredentialAsync: async () => undefined,
    getCredentialData: async () => ({}),
    resolve,
    vars,
    getNodeContext: <T = Record<string, unknown>>() => nodeCtx as T,
    setNodeContext: (data) => Object.assign(nodeCtx, data),
    expressionCtx,
  };
}

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

type Case = {
  type: NodeType;
  parameters?: Record<string, unknown>;
  items?: Record<string, unknown>[];
  inputsByHandle?: Record<string, WorkflowItem[]>;
  runInput?: unknown;
  /** Expect the handler to throw (message substring). */
  expectThrow?: string;
  assert?: (res: NodeResult) => void;
};

function sampleItems(n = 1): Record<string, unknown>[] {
  return Array.from({ length: n }, (_, i) => ({
    id: i + 1,
    n: i + 1,
    name: `item-${i + 1}`,
    ok: true,
  }));
}

async function withTempDir<T>(fn: (dir: string) => Promise<T>): Promise<T> {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "dataflow-smoke-"));
  try {
    return await fn(dir);
  } finally {
    await fs.rm(dir, { recursive: true, force: true });
  }
}

function buildCases(tmp: string): Case[] {
  const fileA = path.join(tmp, "a.txt");
  const fileB = path.join(tmp, "b.txt");
  const fileC = path.join(tmp, "c.txt");
  const fileJson = path.join(tmp, "data.json");
  const fileXlsx = path.join(tmp, "sheet.xlsx");
  const fileGz = path.join(tmp, "a.txt.gz");

  const left = normalizeItems([{ id: 1, a: "x" }, { id: 2, a: "y" }]);
  const right = normalizeItems([{ id: 1, b: "z" }, { id: 3, b: "w" }]);

  return [
    // --- Triggers ---
    {
      type: "webhook",
      runInput: { hello: "world" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).hello === "world", "webhook"),
    },
    {
      type: "schedule",
      assert: (r) => assert(r.items.length >= 1, "schedule"),
    },
    {
      type: "manual",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).triggered === true, "manual"),
    },
    {
      type: "rabbitmqTrigger",
      parameters: { queue: "q1" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "rabbitmq", "rabbitmqTrigger"),
    },
    {
      type: "redisTrigger",
      parameters: { channel: "ch1" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "redis", "redisTrigger"),
    },
    {
      type: "outlookTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "outlook", "outlookTrigger"),
    },
    {
      type: "teamsTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "teams", "teamsTrigger"),
    },
    {
      type: "whatsappTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "whatsapp", "whatsappTrigger"),
    },
    {
      type: "formTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "form", "formTrigger"),
    },
    {
      type: "errorTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "error", "errorTrigger"),
    },
    {
      type: "localFileTrigger",
      parameters: { path: tmp },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "localFile", "localFileTrigger"),
    },
    {
      type: "sseTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "sse", "sseTrigger"),
    },
    {
      type: "rssFeedTrigger",
      parameters: { url: "https://example.com/feed" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "rss", "rssFeedTrigger"),
    },
    {
      type: "emailTriggerImap",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "email", "emailTriggerImap"),
    },
    {
      type: "executeWorkflowTrigger",
      assert: (r) =>
        assert(itemJson(r.items[0] as WorkflowItem).trigger === "executeWorkflow", "executeWorkflowTrigger"),
    },
    {
      type: "workflowTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "workflow", "workflowTrigger"),
    },
    {
      type: "n8nTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "n8n", "n8nTrigger"),
    },
    {
      type: "interval",
      parameters: { seconds: 30 },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "interval", "interval"),
    },
    {
      type: "activationTrigger",
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "activation", "activationTrigger"),
    },
    {
      type: "simulateTrigger",
      parameters: { output: [{ simulated: true, n: 1 }] },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).simulated === true, "simulateTrigger"),
    },
    {
      type: "mcpServerTrigger",
      parameters: { toolName: "run" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "mcpServer", "mcpServerTrigger"),
    },
    {
      type: "evaluationTrigger",
      parameters: { datasetId: "ds1" },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).trigger === "evaluation", "evaluationTrigger"),
    },
    {
      type: "apiEndpoint",
      runInput: { path: "/ping", method: "GET" },
      assert: (r) => assert(r.items.length >= 1, "apiEndpoint"),
    },

    // --- Control ---
    {
      type: "if",
      parameters: { field: "ok", operator: "equals", value: true },
      items: [{ ok: true }, { ok: false }],
      assert: (r) => {
        assert(r.outputs!.true!.length === 1, "if true");
        assert(r.outputs!.false!.length === 1, "if false");
      },
    },
    {
      type: "switch",
      parameters: {
        mode: "rules",
        rules: [{ outputKey: "hi", conditions: [{ field: "n", operator: "gt", value: 10 }] }],
        fallbackOutput: "default",
      },
      items: [{ n: 20 }, { n: 2 }],
      assert: (r) => {
        assert(r.outputs!.hi!.length === 1, "switch hi");
        assert(r.outputs!.default!.length === 1, "switch default");
      },
    },
    {
      type: "merge",
      parameters: { mode: "combineByFields", fields1: "id", fields2: "id", joinMode: "keepMatches" },
      items: [],
      inputsByHandle: { "0": left, "1": right },
      assert: (r) => {
        assert(r.items.length === 1, "merge matches");
        assert(itemJson(r.items[0] as WorkflowItem).b === "z", "merge fields");
      },
    },
    {
      type: "wait",
      parameters: { ms: 1 },
      items: sampleItems(1),
      assert: (r) => assert(r.items.length === 1, "wait"),
    },
    {
      type: "mcpCall",
      parameters: {},
      expectThrow: "tool name is required",
    },
    {
      type: "noOp",
      items: sampleItems(2),
      assert: (r) => assert(r.items.length === 2, "noOp"),
    },
    {
      type: "stopAndError",
      parameters: { message: "boom-test" },
      expectThrow: "boom-test",
    },
    {
      type: "stickyNote",
      items: sampleItems(1),
      assert: (r) => assert(r.items.length === 1, "stickyNote"),
    },
    {
      type: "executeWorkflow",
      parameters: {},
      items: sampleItems(1),
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).warning, "executeWorkflow warning"),
    },
    {
      type: "executeCommand",
      parameters: { command: process.platform === "win32" ? "echo dataflow" : "echo dataflow" },
      assert: (r) => {
        const j = itemJson(r.items[0] as WorkflowItem);
        assert(String(j.stdout).toLowerCase().includes("dataflow"), "executeCommand stdout");
      },
    },

    // --- API ---
    {
      type: "httpRequest",
      parameters: {
        method: "GET",
        url: "data:application/json,{\"ok\":true,\"from\":\"httpRequest\"}",
        responseFormat: "json",
      },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).ok === true, "httpRequest"),
    },
    {
      type: "webhookRespond",
      parameters: { statusCode: 200, responseData: "firstEntryJson" },
      items: [{ answered: true }],
      assert: (r) => {
        assert(r.respond?.statusCode === 200, "webhookRespond status");
        assert((r.respond?.body as { answered?: boolean })?.answered === true, "webhookRespond body");
      },
    },
    {
      type: "apiRespond",
      parameters: { statusCode: 201, body: { created: true } },
      items: [{ x: 1 }],
      assert: (r) => assert(r.respond?.statusCode === 201, "apiRespond"),
    },
    {
      type: "graphql",
      parameters: {},
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).warning, "graphql warning"),
    },
    {
      type: "jwt",
      parameters: { operation: "sign", secret: "test-secret", payload: { sub: "u1" } },
      assert: (r) => assert(typeof itemJson(r.items[0] as WorkflowItem).token === "string", "jwt sign"),
    },
    {
      type: "n8nApi",
      parameters: { operation: "getWorkflows" },
      expectThrow: "n8n baseUrl",
    },
    {
      type: "form",
      parameters: { formId: "f1", fields: { name: "Ada" } },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).formId === "f1", "form"),
    },
    {
      type: "rssRead",
      parameters: {},
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).warning, "rssRead warning"),
    },
    {
      type: "sendEmail",
      parameters: { to: "a@b.c", subject: "hi", text: "body" },
      expectThrow: "SMTP",
    },
    {
      type: "ftp",
      parameters: { path: "/" },
      expectThrow: "FTP host",
    },
    {
      type: "ssh",
      parameters: { host: "localhost", command: "uname" },
      expectThrow: "SSH username",
    },
    {
      type: "git",
      parameters: { command: "git --version" },
      assert: (r) => assert(String(itemJson(r.items[0] as WorkflowItem).stdout).includes("git"), "git"),
    },
    {
      type: "ldap",
      parameters: { operation: "search" },
      expectThrow: "LDAP host",
    },

    // --- Microsoft ---
    {
      type: "mssql",
      parameters: { query: "SELECT 1 AS ok" },
      expectThrow: "MSSQL connectionString",
    },
    {
      type: "microsoftGraph",
      parameters: { path: "/me" },
      expectThrow: "Microsoft Graph",
    },
    {
      type: "outlook",
      parameters: { action: "listMessages" },
      expectThrow: "Microsoft Graph",
    },
    {
      type: "excelOnline",
      expectThrow: "Microsoft Graph",
    },
    {
      type: "sharepoint",
      expectThrow: "Microsoft Graph",
    },
    {
      type: "dynamics",
      parameters: { operation: "GET", type: "ODATA", getQuery: "accounts" },
      expectThrow: "Dataverse",
    },

    // --- Files ---
    {
      type: "writeFile",
      parameters: { path: fileA, content: "hello-smoke" },
      assert: (r) =>
        assert(
          Number((itemJson(r.items[0] as WorkflowItem) as { bytes?: number }).bytes) > 0,
          "writeFile",
        ),
    },
    {
      type: "readFile",
      parameters: { path: fileA },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).content === "hello-smoke", "readFile"),
    },
    {
      type: "listFiles",
      parameters: { path: tmp },
      assert: (r) => assert(r.items.length >= 1, "listFiles"),
    },
    {
      type: "copyFile",
      parameters: { from: fileA, to: fileB },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).to === fileB, "copyFile"),
    },
    {
      type: "moveFile",
      parameters: { from: fileB, to: fileC },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).to === fileC, "moveFile"),
    },
    {
      type: "deleteFile",
      parameters: { path: fileC },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).deleted === true, "deleteFile"),
    },
    {
      type: "csv",
      parameters: { operation: "parse", content: "name,n\nAda,1\nBob,2" },
      assert: (r) => {
        assert(r.items.length === 2, "csv rows");
        assert(itemJson(r.items[0] as WorkflowItem).name === "Ada", "csv parse");
      },
    },
    {
      type: "json",
      parameters: { operation: "parse", content: '[{"a":1},{"a":2}]' },
      assert: (r) => assert(r.items.length === 2 && itemJson(r.items[0] as WorkflowItem).a === 1, "json"),
    },
    {
      type: "xml",
      parameters: { operation: "wrap", root: "items" },
      items: [{ name: "Ada" }],
      assert: (r) => {
        const c = String(itemJson(r.items[0] as WorkflowItem).content);
        assert(c.includes("Ada") && c.includes("name"), "xml wrap");
      },
    },
    {
      type: "excel",
      parameters: { operation: "write", path: fileXlsx },
      items: [{ name: "Ada", n: 1 }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).rows === 1, "excel write"),
    },
    {
      type: "zip",
      parameters: { operation: "gzip", from: fileA, to: fileGz },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).operation === "gzip", "zip"),
    },
    {
      type: "compression",
      parameters: { operation: "compress", field: "name" },
      items: [{ name: "Ada" }],
      assert: (r) => {
        assert(itemJson(r.items[0] as WorkflowItem).compression === "compress", "compression");
        assert((r.items[0] as WorkflowItem).binary, "compression binary");
      },
    },
    {
      type: "convertToFile",
      parameters: { path: fileJson, fileName: "data.json", content: '{"ok":true}' },
      assert: (r) =>
        assert(
          Number((itemJson(r.items[0] as WorkflowItem) as { bytes?: number }).bytes) > 0,
          "convertToFile",
        ),
    },
    {
      type: "extractFromFile",
      parameters: { path: fileJson },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).ok === true, "extractFromFile"),
    },
    {
      type: "readPdf",
      parameters: {},
      expectThrow: "path or binary",
    },
    {
      type: "moveBinaryData",
      parameters: { mode: "jsonToBinary" },
      items: [{ hello: "bin" }],
      assert: (r) => assert((r.items[0] as WorkflowItem).binary, "moveBinaryData"),
    },
    {
      type: "editImage",
      parameters: { operation: "noop" },
      items: sampleItems(1),
      assert: (r) => assert(r.items.length === 1, "editImage noop"),
    },
    {
      type: "icalendar",
      parameters: { title: "Smoke Event", start: "2026-01-01T10:00:00.000Z" },
      assert: (r) => assert(String(itemJson(r.items[0] as WorkflowItem).ics).includes("VEVENT"), "icalendar"),
    },

    // --- Data ---
    {
      type: "set",
      parameters: { values: { tagged: true }, include: "all" },
      items: [{ a: 1 }],
      assert: (r) => {
        assert(itemJson(r.items[0] as WorkflowItem).tagged === true, "set tagged");
        assert(itemJson(r.items[0] as WorkflowItem).a === 1, "set keep");
      },
    },
    {
      type: "editFields",
      parameters: { mapping: { name: "fullName" } },
      items: [{ name: "Ada" }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).fullName === "Ada", "editFields"),
    },
    {
      type: "filter",
      parameters: { field: "n", operator: "gt", value: 1 },
      items: [{ n: 1 }, { n: 3 }],
      assert: (r) => {
        assert(r.outputs!.kept!.length === 1, "filter kept");
        assert(r.outputs!.discarded!.length === 1, "filter discarded");
      },
    },
    {
      type: "sort",
      parameters: { field: "n", direction: "desc" },
      items: [{ n: 1 }, { n: 3 }, { n: 2 }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).n === 3, "sort"),
    },
    {
      type: "limit",
      parameters: { max: 2, keep: "first" },
      items: sampleItems(5),
      assert: (r) => assert(r.items.length === 2, "limit"),
    },
    {
      type: "splitInBatches",
      parameters: { batchSize: 2 },
      items: sampleItems(3),
      assert: (r) => {
        assert(r.outputs!.loop!.length === 2, "splitInBatches loop");
        assert(r.continueLoop === true, "splitInBatches continue");
      },
    },
    {
      type: "splitOut",
      parameters: { field: "tags", include: "noOtherFields" },
      items: [{ tags: ["a", "b"] }],
      assert: (r) => assert(r.items.length === 2, "splitOut"),
    },
    {
      type: "aggregate",
      parameters: { mode: "individualFields", field: "n" },
      items: [{ n: 1 }, { n: 2 }],
      assert: (r) => assert(Array.isArray(itemJson(r.items[0] as WorkflowItem).n), "aggregate"),
    },
    {
      type: "summarize",
      parameters: {
        aggregations: [{ field: "n", aggregation: "sum", outputFieldName: "total" }],
      },
      items: [{ n: 1 }, { n: 4 }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).total === 5, "summarize"),
    },
    {
      type: "itemLists",
      parameters: { operation: "flatten", field: "batch" },
      items: [{ batch: [{ v: 1 }, { v: 2 }] }],
      assert: (r) => assert(r.items.length === 2, "itemLists"),
    },
    {
      type: "renameKeys",
      parameters: { mapping: { name: "label" } },
      items: [{ name: "Ada" }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).label === "Ada", "renameKeys"),
    },
    {
      type: "removeDuplicates",
      parameters: { compare: "selectedFields", fields: ["id"] },
      items: [{ id: 1 }, { id: 1 }, { id: 2 }],
      assert: (r) => {
        assert(r.outputs!.kept!.length === 2, "dedupe kept");
        assert(r.outputs!.discarded!.length === 1, "dedupe discarded");
      },
    },
    {
      type: "compareDatasets",
      parameters: { field: "id" },
      items: [],
      inputsByHandle: {
        "0": normalizeItems([{ id: 1 }, { id: 2 }]),
        "1": normalizeItems([{ id: 2 }, { id: 3 }]),
      },
      assert: (r) => {
        assert(r.outputs!.both!.length === 1, "compare both");
        assert(r.outputs!.onlyA!.length === 1, "compare onlyA");
        assert(r.outputs!.onlyB!.length === 1, "compare onlyB");
      },
    },
    {
      type: "code",
      parameters: {
        mode: "runOnceForAllItems",
        code: "return items.map(i => ({ json: { ...i.json, x: 1 } }));",
      },
      items: [{ a: 1 }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).x === 1, "code"),
    },
    {
      type: "aiTransform",
      parameters: { prompt: "normalize" },
      items: sampleItems(1),
      expectThrow: "OPENAI_API_KEY",
    },
    {
      type: "dateTime",
      parameters: { operation: "now" },
      assert: (r) => assert(typeof itemJson(r.items[0] as WorkflowItem).iso === "string", "dateTime"),
    },
    {
      type: "crypto",
      parameters: { operation: "hash", algorithm: "sha256", field: "name" },
      items: [{ name: "Ada" }],
      assert: (r) => assert(typeof itemJson(r.items[0] as WorkflowItem).hash === "string", "crypto"),
    },
    {
      type: "html",
      parameters: { template: "<b>{{name}}</b>" },
      items: [{ name: "Ada" }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).html === "<b>Ada</b>", "html"),
    },
    {
      type: "htmlExtract",
      parameters: { selector: "title" },
      items: [{ html: "<html><title>Hi</title></html>" }],
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).extracted === "Hi", "htmlExtract"),
    },
    {
      type: "markdown",
      parameters: { field: "text", mode: "toHtml" },
      items: [{ text: "# Title" }],
      assert: (r) => assert(String(itemJson(r.items[0] as WorkflowItem).html).includes("<h1>"), "markdown"),
    },
    {
      type: "totp",
      parameters: { secret: "smoke" },
      assert: (r) => assert(String(itemJson(r.items[0] as WorkflowItem).token).length === 6, "totp"),
    },
    {
      type: "timeSaved",
      parameters: { minutesSaved: 3, mode: "once" },
      items: sampleItems(2),
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).minutesSaved === 3, "timeSaved"),
    },
    {
      type: "dataTable",
      parameters: { columns: ["id", "name"] },
      items: [{ id: 1, name: "Ada", extra: true }],
      assert: (r) => {
        const rows = itemJson(r.items[0] as WorkflowItem).rows as Record<string, unknown>[];
        assert(rows.length === 1 && rows[0]!.name === "Ada" && rows[0]!.extra === undefined, "dataTable");
      },
    },
    {
      type: "executionData",
      runInput: { from: "run" },
      items: sampleItems(2),
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).itemCount === 2, "executionData"),
    },
    {
      type: "simulate",
      parameters: { output: [{ simulated: true }] },
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).simulated === true, "simulate"),
    },
    {
      type: "debugHelper",
      parameters: { category: "doNothing" },
      items: sampleItems(1),
      assert: (r) => assert(r.items.length === 1, "debugHelper"),
    },
    {
      type: "evaluation",
      parameters: { metric: "accuracy", value: 0.9 },
      items: sampleItems(1),
      assert: (r) => assert(itemJson(r.items[0] as WorkflowItem).evaluation, "evaluation"),
    },
  ];
}

async function runCase(c: Case): Promise<void> {
  const handler = registry[c.type];
  assert(handler, `no registry handler for ${c.type}`);
  const items = normalizeItems(c.items ?? []);
  const nodeCtx = ctx(c.type, c.parameters ?? {}, items, {
    inputsByHandle: c.inputsByHandle,
    runInput: c.runInput,
  });

  if (c.expectThrow) {
    let threw = false;
    try {
      await handler(nodeCtx);
    } catch (err) {
      threw = true;
      assert(
        String((err as Error).message).includes(c.expectThrow),
        `${c.type} throw message: ${(err as Error).message}`,
      );
    }
    assert(threw, `${c.type} expected throw`);
    return;
  }

  const res = await handler(nodeCtx);
  c.assert?.(res);
}

export async function runSmokeAllNodes() {
  console.log("smokeAllNodes: start");

  const registryTypes = Object.keys(registry).sort() as NodeType[];
  const dayOneTypes = new Set(DAY_ONE_NODE_TYPES.map((n) => n.type));

  await withTempDir(async (tmp) => {
    // Seed files used by later file cases (writeFile case also creates a.txt).
    await fs.writeFile(path.join(tmp, "seed.txt"), "seed", "utf8");

    const cases = buildCases(tmp);
    const covered = new Set(cases.map((c) => c.type));

    const missingInCases = registryTypes.filter((t) => !covered.has(t));
    assert(
      missingInCases.length === 0,
      `missing test cases for: ${missingInCases.join(", ")}`,
    );

    const unknownCases = [...covered].filter((t) => !registry[t]);
    assert(
      unknownCases.length === 0,
      `cases for unregistered types: ${unknownCases.join(", ")}`,
    );

    // File cases depend on order within buildCases (write → read → copy → …).
    for (const c of cases) {
      process.stdout.write(`  ${c.type}… `);
      await runCase(c);
      console.log("ok");
    }

    const missingDayOne = DAY_ONE_NODE_TYPES.map((n) => n.type).filter(
      (t) => !registry[t],
    );
    assert(
      missingDayOne.length === 0,
      `DAY_ONE types missing from registry: ${missingDayOne.join(", ")}`,
    );

    const extraRegistry = registryTypes.filter((t) => !dayOneTypes.has(t));
    if (extraRegistry.length) {
      console.log(
        `smokeAllNodes: registry extras not in DAY_ONE_NODE_TYPES: ${extraRegistry.join(", ")}`,
      );
    }

    console.log(
      `smokeAllNodes: all ${cases.length} node types passed (registry=${registryTypes.length})`,
    );
  });
}

const isDirect =
  typeof process !== "undefined" &&
  Boolean(process.argv[1]?.includes("smokeAllNodes"));
if (isDirect) {
  runSmokeAllNodes().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
