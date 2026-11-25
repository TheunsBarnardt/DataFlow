/**
 * In-process smoke for n8n-basics / full-parity engine (no Mongo required).
 * Run: npx tsx src/engine/smokeBasics.ts  (from server workspace)
 */
import type { WorkflowNode } from "@dataflow/shared";
import {
  setNode,
  filterNode,
  sortNode,
  limitNode,
  codeNode,
  aggregateNode,
  splitInBatchesNode,
} from "./nodes/data.js";
import { ifNode, mergeNode, switchNode } from "./nodes/control.js";
import {
  splitOutNode,
  removeDuplicatesNode,
  summarizeNode,
} from "./nodes/core.js";
import { normalizeItems, itemJson, type WorkflowItem } from "./items.js";
import {
  resolveExpressionsInValue,
  type ExpressionRunContext,
  type RunStoreEntry,
} from "./resolveExpressions.js";
import type { NodeContext } from "./types.js";

function ctx(
  type: string,
  parameters: Record<string, unknown>,
  items: WorkflowItem[],
  inputsByHandle?: Record<string, WorkflowItem[]>,
  runStore?: Map<string, RunStoreEntry>,
): NodeContext {
  const vars: Record<string, string> = { ENV: "test" };
  const store = runStore ?? new Map();
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
      id: "n",
      type: type as WorkflowNode["type"],
      name: type,
      position: { x: 0, y: 0 },
      parameters: resolveExpressionsInValue(parameters, vars, undefined, undefined, {
        vars,
        runStore: store,
        throwOnError: false,
      }) as Record<string, unknown>,
    },
    items,
    inputsByHandle: inputsByHandle ?? { main: items, "0": items },
    runInput: undefined,
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

export async function runSmokeBasics() {
  console.log("smokeBasics: start");

  // Typed whole-value expression
  const typed = resolveExpressionsInValue(
    "={{ $json.n }}" as string,
    {},
    { json: { n: 5 } },
  ) as unknown;
  assert(typed === 5, `typed expr got ${typed} (${typeof typed})`);

  // Inline + vars
  const expr = resolveExpressionsInValue(
    "hello {{ $json.name }} $vars.ENV" as string,
    { ENV: "prod" },
    { json: { name: "Ada" } },
  );
  assert(String(expr) === "hello Ada prod", `expr got ${expr}`);

  // $('NodeName')
  const store = new Map<string, RunStoreEntry>();
  store.set("Webhook", {
    nodeId: "w1",
    nodeName: "Webhook",
    outputs: { main: [{ json: { hello: "world" } }] },
  });
  const cross = resolveExpressionsInValue(
    "={{ $('Webhook').item.hello }}" as string,
    {},
    { json: {} },
    [{ json: {} }],
    { vars: {}, runStore: store, throwOnError: true },
  ) as unknown;
  assert(cross === "world", `cross-node got ${cross}`);

  // Set
  const setRes = await setNode(
    ctx(
      "set",
      { values: { ok: true }, include: "all" },
      normalizeItems([{ a: 1 }]),
    ),
  );
  assert(itemJson(setRes.items[0]!).ok === true, "set ok");
  assert(itemJson(setRes.items[0]!).a === 1, "set keep a");

  // Set except
  const setEx = await setNode(
    ctx(
      "set",
      {
        include: "except",
        excludeFields: ["secret"],
        values: { marked: true },
      },
      normalizeItems([{ secret: 1, keep: 2 }]),
    ),
  );
  assert(itemJson(setEx.items[0]!).keep === 2, "set except keep");
  assert(itemJson(setEx.items[0]!).secret === undefined, "set except drop");

  // Filter dual output
  const filterRes = await filterNode(
    ctx(
      "filter",
      { field: "n", operator: "gt", value: 5 },
      normalizeItems([{ n: 3 }, { n: 9 }, { n: 6 }]),
    ),
  );
  assert(filterRes.outputs!.kept!.length === 2, "filter kept");
  assert(filterRes.outputs!.discarded!.length === 1, "filter discarded");

  // If multi-condition
  const ifRes = await ifNode(
    ctx(
      "if",
      {
        conditions: {
          combinator: "AND",
          conditions: [
            { field: "n", operator: "gte", value: 5 },
            { field: "ok", operator: "equals", value: true },
          ],
        },
      },
      normalizeItems([
        { n: 5, ok: true },
        { n: 9, ok: false },
        { n: 1, ok: true },
      ]),
    ),
  );
  assert(ifRes.outputs!.true!.length === 1, "if true");
  assert(ifRes.outputs!.false!.length === 2, "if false");

  // Switch rules
  const sw = await switchNode(
    ctx(
      "switch",
      {
        mode: "rules",
        rules: [
          {
            outputKey: "hi",
            conditions: [{ field: "n", operator: "gt", value: 10 }],
          },
        ],
        fallbackOutput: "default",
      },
      normalizeItems([{ n: 20 }, { n: 2 }]),
    ),
  );
  assert(sw.outputs!.hi!.length === 1, "switch hi");
  assert(sw.outputs!.default!.length === 1, "switch default");

  // Sort + Limit
  const sorted = await sortNode(
    ctx(
      "sort",
      { field: "n", direction: "desc" },
      normalizeItems([{ n: 1 }, { n: 3 }, { n: 2 }]),
    ),
  );
  assert(itemJson(sorted.items[0]!).n === 3, "sort");
  const limited = await limitNode(
    ctx("limit", { max: 2, keep: "first" }, sorted.items as WorkflowItem[]),
  );
  assert(limited.items.length === 2, "limit");

  // Merge combineByFields
  const left = normalizeItems([{ id: 1, a: "x" }, { id: 2, a: "y" }]);
  const right = normalizeItems([{ id: 1, b: "z" }]);
  const merged = await mergeNode(
    ctx(
      "merge",
      { mode: "combineByFields", fields1: "id", fields2: "id", joinMode: "keepMatches" },
      [...left, ...right],
      { "0": left, "1": right },
    ),
  );
  assert(merged.items.length === 1, "merge match");
  assert(itemJson(merged.items[0]!).b === "z", "merge fields");

  // Merge combineAll
  const cart = await mergeNode(
    ctx(
      "merge",
      { mode: "combineAll" },
      [],
      {
        "0": normalizeItems([{ a: 1 }, { a: 2 }]),
        "1": normalizeItems([{ b: 3 }]),
      },
    ),
  );
  assert(cart.items.length === 2, "combineAll");

  // Split Out
  const split = await splitOutNode(
    ctx(
      "splitOut",
      { field: "tags", include: "noOtherFields" },
      normalizeItems([{ tags: ["a", "b"] }]),
    ),
  );
  assert(split.items.length === 2, "splitOut");

  // Remove duplicates
  const dedupe = await removeDuplicatesNode(
    ctx(
      "removeDuplicates",
      { compare: "selectedFields", fields: ["id"] },
      normalizeItems([{ id: 1 }, { id: 1 }, { id: 2 }]),
    ),
  );
  assert(dedupe.outputs!.kept!.length === 2, "dedupe kept");
  assert(dedupe.outputs!.discarded!.length === 1, "dedupe discarded");

  // Aggregate + Summarize
  const agg = await aggregateNode(
    ctx(
      "aggregate",
      { mode: "individualFields", field: "n" },
      normalizeItems([{ n: 1 }, { n: 2 }]),
    ),
  );
  assert(Array.isArray(itemJson(agg.items[0]!).n), "aggregate");
  const sum = await summarizeNode(
    ctx(
      "summarize",
      {
        aggregations: [
          { field: "n", aggregation: "sum", outputFieldName: "total" },
        ],
      },
      normalizeItems([{ n: 1 }, { n: 4 }]),
    ),
  );
  assert(itemJson(sum.items[0]!).total === 5, "summarize");

  // Code
  const code = await codeNode(
    ctx(
      "code",
      {
        mode: "runOnceForAllItems",
        code: "return items.map(i => ({ json: { ...i.json, x: 1 } }));",
      },
      normalizeItems([{ a: 1 }]),
    ),
  );
  assert(itemJson(code.items[0]!).x === 1, "code");

  // SplitInBatches loop/done
  const batchItems = normalizeItems([
    { i: 1 },
    { i: 2 },
    { i: 3 },
    { i: 4 },
  ]);
  const batchCtx = ctx("splitInBatches", { batchSize: 2 }, batchItems);
  const b1 = await splitInBatchesNode(batchCtx);
  assert(b1.outputs!.loop!.length === 2, "batch1 loop");
  assert(b1.continueLoop === true, "batch1 continue");
  const b2 = await splitInBatchesNode(batchCtx);
  assert(b2.outputs!.loop!.length === 2, "batch2 loop");
  assert(b2.continueLoop === false, "batch2 done");

  // Expression in set
  const setExpr = await setNode(
    ctx(
      "set",
      { values: { greeting: "={{ 'hi-' + $json.name }}" }, include: "all" },
      normalizeItems([{ name: "Ada" }]),
    ),
  );
  assert(
    itemJson(setExpr.items[0]!).greeting === "hi-Ada",
    `set expr got ${itemJson(setExpr.items[0]!).greeting}`,
  );

  // n8n-style map: $json expressions must survive a no-item pre-pass when
  // runStore already has upstream outputs, then resolve per item.
  const upstreamStore = new Map<string, RunStoreEntry>();
  upstreamStore.set("MSSQL", {
    nodeId: "sql1",
    nodeName: "MSSQL",
    outputs: { main: [{ json: { Id: 1 } }] },
  });
  const deferred = resolveExpressionsInValue(
    "={{ $json.Company }}" as string,
    {},
    undefined,
    undefined,
    { vars: {}, runStore: upstreamStore, throwOnError: true },
  );
  assert(
    deferred === "={{ $json.Company }}",
    `expected deferred $json expr, got ${JSON.stringify(deferred)}`,
  );
  const mappedMulti = await setNode(
    ctx(
      "set",
      {
        values: [{ name: "company", value: "={{ $json.Company }}", type: "string" }],
        include: "none",
      },
      normalizeItems([
        { Company: "Wealth" },
        { Company: "Sasfin" },
      ]),
      undefined,
      upstreamStore,
    ),
  );
  assert(
    itemJson(mappedMulti.items[0]!).company === "Wealth" &&
      itemJson(mappedMulti.items[1]!).company === "Sasfin",
    `per-item map got ${JSON.stringify(mappedMulti.items.map(itemJson))}`,
  );

  console.log("smokeBasics: all passed");
}

const isDirect =
  typeof process !== "undefined" &&
  Boolean(process.argv[1]?.includes("smokeBasics"));
if (isDirect) {
  runSmokeBasics().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
