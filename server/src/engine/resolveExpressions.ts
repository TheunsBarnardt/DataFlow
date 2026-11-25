/**
 * n8n-style expression resolution (clean-room).
 * Supports $json, $binary, $input, $vars, $now, $today, $parameter,
 * $('NodeName') / $node["Name"], and ={{ }} with native return types.
 */

import type { WorkflowItem } from "./items.js";
import { itemJson } from "./items.js";
import { resolveVarsInString } from "./resolveVars.js";

export class ExpressionError extends Error {
  itemIndex?: number;
  constructor(message: string, opts?: { itemIndex?: number }) {
    super(message);
    this.name = "ExpressionError";
    this.itemIndex = opts?.itemIndex;
  }
}

export type RunStoreEntry = {
  nodeId: string;
  nodeName: string;
  outputs: Record<string, WorkflowItem[]>;
};

export type ExpressionRunContext = {
  vars: Record<string, string>;
  /** Completed node outputs keyed by node name (and id as fallback). */
  runStore: Map<string, RunStoreEntry>;
  /** Current node parameters (for $parameter). */
  parameters?: Record<string, unknown>;
  itemIndex?: number;
  runIndex?: number;
  /** When false, throw on expression errors instead of empty string. Default true for throw. */
  throwOnError?: boolean;
};

function getPath(obj: unknown, path: string): unknown {
  if (!path) return obj;
  return path.split(".").reduce<unknown>((acc, key) => {
    if (acc && typeof acc === "object") {
      return (acc as Record<string, unknown>)[key];
    }
    return undefined;
  }, obj);
}

function pathToString(value: unknown): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function startOfToday(): Date {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d;
}

function nodeLookup(
  runStore: Map<string, RunStoreEntry>,
  name: string,
): RunStoreEntry | undefined {
  if (runStore.has(name)) return runStore.get(name);
  for (const entry of runStore.values()) {
    if (entry.nodeName === name || entry.nodeId === name) return entry;
  }
  return undefined;
}

function makeNodeAccessor(entry: RunStoreEntry | undefined) {
  const main = entry?.outputs.main ?? entry?.outputs.true ?? [];
  return {
    item: main[0] ? itemJson(main[0]) : {},
    first: () => (main[0] ? itemJson(main[0]) : {}),
    last: () =>
      main.length ? itemJson(main[main.length - 1]!) : {},
    all: () => main.map((i) => itemJson(i)),
    json: main[0] ? itemJson(main[0]) : {},
    binary: main[0]?.binary ?? {},
    items: main,
    isExecuted: Boolean(entry),
  };
}

export type ProxyBundle = {
  $json: Record<string, unknown>;
  $binary: Record<string, unknown>;
  $item: WorkflowItem;
  $itemIndex: number;
  $runIndex: number;
  $vars: Record<string, string>;
  $input: {
    item: Record<string, unknown>;
    first: () => Record<string, unknown>;
    last: () => Record<string, unknown>;
    all: () => Record<string, unknown>[];
    params: Record<string, unknown>;
  };
  $parameter: Record<string, unknown>;
  $now: Date;
  $today: Date;
  $: (name: string) => ReturnType<typeof makeNodeAccessor>;
  $node: Record<string, ReturnType<typeof makeNodeAccessor>>;
};

export function buildExpressionProxy(
  item: WorkflowItem | undefined,
  allItems: WorkflowItem[],
  ctx: ExpressionRunContext,
): ProxyBundle {
  const json = item ? itemJson(item) : {};
  const binary = (item?.binary as Record<string, unknown>) ?? {};
  const itemsJson = allItems.map((i) => itemJson(i));
  const itemIndex = ctx.itemIndex ?? 0;
  const params = ctx.parameters ?? {};

  const $node: Record<string, ReturnType<typeof makeNodeAccessor>> = {};
  for (const entry of ctx.runStore.values()) {
    $node[entry.nodeName] = makeNodeAccessor(entry);
  }

  const $ = (name: string) => {
    const entry = nodeLookup(ctx.runStore, name);
    if (!entry) {
      throw new ExpressionError(`Referenced node "${name}" hasn't been executed`, {
        itemIndex,
      });
    }
    return makeNodeAccessor(entry);
  };

  return {
    $json: json,
    $binary: binary,
    $item: item ?? { json: {} },
    $itemIndex: itemIndex,
    $runIndex: ctx.runIndex ?? 0,
    $vars: ctx.vars,
    $input: {
      item: json,
      first: () => itemsJson[0] ?? {},
      last: () => itemsJson[itemsJson.length - 1] ?? {},
      all: () => itemsJson,
      params,
    },
    $parameter: params,
    $now: new Date(),
    $today: startOfToday(),
    $,
    $node,
  };
}

function evalExpression(
  expr: string,
  proxy: ProxyBundle,
  itemIndex?: number,
): unknown {
  try {
    // eslint-disable-next-line no-new-func
    const fn = new Function(
      "$json",
      "$binary",
      "$item",
      "$itemIndex",
      "$runIndex",
      "$vars",
      "$input",
      "$parameter",
      "$now",
      "$today",
      "$",
      "$node",
      `"use strict"; return (${expr});`,
    );
    return fn(
      proxy.$json,
      proxy.$binary,
      proxy.$item,
      proxy.$itemIndex,
      proxy.$runIndex,
      proxy.$vars,
      proxy.$input,
      proxy.$parameter,
      proxy.$now,
      proxy.$today,
      proxy.$,
      proxy.$node,
    );
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new ExpressionError(`Expression failed: ${msg}`, { itemIndex });
  }
}

const WHOLE_EXPR = /^=\{\{\s*([\s\S]*?)\s*\}\}$/;
const INLINE_EXPR = /=\{\{\s*([\s\S]*?)\s*\}\}|\{\{\s*([\s\S]*?)\s*\}\}/g;

/**
 * Resolve a single string. If the entire value is `={{ expr }}`, return the
 * native typed result. Otherwise interpolate expressions as strings.
 */
export function resolveExpressionString(
  input: string,
  vars: Record<string, string>,
  item?: WorkflowItem,
  allItems?: WorkflowItem[],
  runCtx?: ExpressionRunContext,
): unknown {
  const ctx: ExpressionRunContext = runCtx ?? {
    vars,
    runStore: new Map(),
    throwOnError: false,
  };
  let s = resolveVarsInString(input, vars);

  // Whole-value expression → native type
  const whole = s.trim().match(WHOLE_EXPR);
  if (whole) {
    const expr = whole[1]!.trim();
    // Item-scoped refs must wait for per-item resolve (n8n-style mapping).
    // Previously we only deferred when runStore was empty — once an upstream
    // node had run, `={{ $json.field }}` evaluated against `{}` and wiped the
    // expression before handlers could map each item.
    const needsItem =
      /\$json\b|\$input\b|\$item\b|\$binary\b|\$itemIndex\b/.test(expr);
    if (!item && (needsItem || !ctx.runStore.size)) {
      return input;
    }
    const proxy = buildExpressionProxy(item, allItems ?? (item ? [item] : []), {
      ...ctx,
      vars,
    });
    try {
      return evalExpression(expr, proxy, ctx.itemIndex);
    } catch (err) {
      if (ctx.throwOnError !== false) throw err;
      return "";
    }
  }

  if (!item && !/=\{\{|\$\(|\$node|\$json|\$input|\$binary|\$now|\$today/.test(s)) {
    return s;
  }

  if (!item) return s;

  const proxy = buildExpressionProxy(item, allItems ?? [item], { ...ctx, vars });

  try {
    s = s.replace(INLINE_EXPR, (_full, a: string | undefined, b: string | undefined) => {
      const expr = (a ?? b ?? "").trim();
      if (!expr) return "";
      return pathToString(evalExpression(expr, proxy, ctx.itemIndex));
    });

    // $('NodeName').json.field style already handled inside ={{ }}
    // Bare $json.path outside expressions
    s = s.replace(/\$json\.([A-Za-z0-9_.]+)/g, (_full, path: string) =>
      pathToString(getPath(proxy.$json, path)),
    );

    return s;
  } catch (err) {
    if (ctx.throwOnError !== false) throw err;
    return "";
  }
}

/** Deep-walk value; resolve string leaves with optional item + run context. */
export function resolveExpressionsInValue<T>(
  value: T,
  vars: Record<string, string>,
  item?: WorkflowItem,
  allItems?: WorkflowItem[],
  runCtx?: ExpressionRunContext,
): T {
  if (typeof value === "string") {
    return resolveExpressionString(
      value,
      vars,
      item,
      allItems,
      runCtx,
    ) as T;
  }
  if (Array.isArray(value)) {
    return value.map((v) =>
      resolveExpressionsInValue(v, vars, item, allItems, runCtx),
    ) as T;
  }
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = resolveExpressionsInValue(v, vars, item, allItems, runCtx);
    }
    return out as T;
  }
  return value;
}
