import { createHash, createHmac, randomBytes } from "node:crypto";
import type { NodeHandler } from "../types.js";
import {
  ensureItems,
  getJsonPath,
  itemJson,
  makeItem,
  setJsonPath,
  toItem,
  type WorkflowItem,
} from "../items.js";
import {
  matchConditionGroup,
  type Condition,
  type ConditionGroup,
} from "../conditions.js";

function coerceValue(value: unknown, type?: string): unknown {
  if (type === "number") return Number(value);
  if (type === "boolean") {
    if (typeof value === "boolean") return value;
    return String(value).toLowerCase() === "true" || value === "1";
  }
  if (type === "array") {
    if (Array.isArray(value)) return value;
    if (typeof value === "string") {
      try {
        const parsed = JSON.parse(value);
        return Array.isArray(parsed) ? parsed : [value];
      } catch {
        return value.split(",").map((s) => s.trim());
      }
    }
    return [value];
  }
  if (type === "object") {
    if (value && typeof value === "object") return value;
    if (typeof value === "string") {
      try {
        return JSON.parse(value);
      } catch {
        return { value };
      }
    }
    return { value };
  }
  return value;
}

/** Edit Fields (Set) — manual/raw + include all/none/selected/except + binary. */
export const setNode: NodeHandler = async ({ node, items, resolve }) => {
  const base = items.length ? items : [makeItem({})];
  const mode = String(node.parameters.mode ?? "manual");
  const include = String(
    node.parameters.include ??
      (node.parameters.keepOnlySet === true ? "none" : "all"),
  );
  const includeFields = (node.parameters.includeFields as string[]) ?? [];
  const excludeFields =
    (node.parameters.excludeFields as string[]) ??
    (node.parameters.includeExcept as string[]) ??
    [];
  const dotNotation = node.parameters.dotNotation !== false;
  const stripBinary = Boolean(node.parameters.stripBinary);
  const valuesParam = node.parameters.values;

  return {
    items: base.map((item, idx) => {
      const resolvedValues = resolve(
        (valuesParam as Record<string, unknown>) ?? {},
        item,
      ) as Record<string, unknown>;

      let newJson: Record<string, unknown> = {};
      if (include === "all") {
        newJson = { ...itemJson(item) };
      } else if (include === "selected") {
        for (const f of includeFields) {
          const v = getJsonPath(item, f);
          if (v !== undefined) {
            if (dotNotation) setJsonPath(newJson, f, v);
            else newJson[f] = v;
          }
        }
      } else if (include === "except" || include === "allExcept") {
        newJson = { ...itemJson(item) };
        for (const f of excludeFields) {
          if (f.includes(".") && dotNotation) {
            const parts = f.split(".");
            let cur: Record<string, unknown> | undefined = newJson;
            for (let i = 0; i < parts.length - 1; i++) {
              const p = cur?.[parts[i]!];
              if (!p || typeof p !== "object") {
                cur = undefined;
                break;
              }
              cur = p as Record<string, unknown>;
            }
            if (cur) delete cur[parts[parts.length - 1]!];
          } else {
            delete newJson[f];
          }
        }
      }
      // include === "none" → start empty

      if (mode === "raw") {
        const raw = resolve(String(node.parameters.jsonOutput ?? "{}"), item);
        try {
          const parsed =
            typeof raw === "object" && raw
              ? raw
              : JSON.parse(String(raw));
          if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
            newJson =
              include === "none"
                ? { ...(parsed as Record<string, unknown>) }
                : { ...newJson, ...(parsed as Record<string, unknown>) };
          }
        } catch {
          /* keep newJson */
        }
      } else if (Array.isArray(valuesParam)) {
        for (const row of valuesParam as Array<{
          name?: string;
          value?: unknown;
          type?: string;
        }>) {
          const name = String(row.name ?? "");
          if (!name) continue;
          const val = coerceValue(resolve(row.value, item), row.type);
          if (dotNotation) setJsonPath(newJson, name, val);
          else newJson[name] = val;
        }
      } else {
        for (const [key, val] of Object.entries(resolvedValues)) {
          if (dotNotation) setJsonPath(newJson, key, val);
          else newJson[key] = val;
        }
      }

      const out = makeItem(newJson, item, idx);
      if (stripBinary) delete out.binary;
      return out;
    }),
  };
};

/** Rename/move fields via from→to mapping. */
export const editFieldsNode: NodeHandler = async ({ node, items }) => {
  const mapping = (node.parameters.mapping as Record<string, string>) ?? {};
  return {
    items: items.map((item, idx) => {
      const next = { ...itemJson(item) };
      for (const [from, to] of Object.entries(mapping)) {
        const val = getJsonPath(item, from);
        setJsonPath(next, to, val);
        if (from !== to) {
          const parts = from.split(".");
          if (parts.length === 1) delete next[from];
          else {
            // best-effort delete leaf
            let cur: Record<string, unknown> | undefined = next;
            for (let i = 0; i < parts.length - 1; i++) {
              const p = cur?.[parts[i]!];
              if (!p || typeof p !== "object") {
                cur = undefined;
                break;
              }
              cur = p as Record<string, unknown>;
            }
            if (cur) delete cur[parts[parts.length - 1]!];
          }
        }
      }
      return makeItem(next, item, idx);
    }),
  };
};

/** Filter with AND/OR conditions → kept + discarded outputs. */
export const filterNode: NodeHandler = async ({ node, items, resolve }) => {
  const group = (node.parameters.conditions as ConditionGroup) ?? undefined;
  const combinator = String(
    group?.combinator ?? node.parameters.combinator ?? "AND",
  ).toUpperCase();
  const ignoreCase = Boolean(group?.ignoreCase ?? node.parameters.ignoreCase);

  const kept: WorkflowItem[] = [];
  const discarded: WorkflowItem[] = [];
  for (const item of items) {
    const resolvedGroup: ConditionGroup = {
      combinator: combinator as "AND" | "OR",
      ignoreCase,
      conditions: (
        group?.conditions ??
        (Array.isArray(node.parameters.conditions)
          ? (node.parameters.conditions as Condition[])
          : [])
      ).map((c) => {
        const fieldStr = c.field != null ? String(c.field) : "";
        const fieldIsExpr =
          /\{\{/.test(fieldStr) || fieldStr.startsWith("=");
        return {
          ...c,
          leftValue:
            c.leftValue !== undefined
              ? resolve(c.leftValue, item)
              : fieldIsExpr
                ? resolve(fieldStr, item)
                : fieldStr
                  ? getJsonPath(item, fieldStr)
                  : undefined,
          value: resolve(c.value, item),
          rightValue:
            c.rightValue !== undefined
              ? resolve(c.rightValue, item)
              : undefined,
        };
      }),
    };
    const ok = matchConditionGroup(item, resolvedGroup, {
      field: node.parameters.field,
      operator: node.parameters.operator,
      value: resolve(node.parameters.value, item),
      combinator,
    });
    if (ok) kept.push(item);
    else discarded.push(item);
  }
  return {
    items: kept,
    outputs: { main: kept, kept, discarded },
  };
};

/** Multi-field sort — simple | random | code. */
export const sortNode: NodeHandler = async ({ node, items }) => {
  const type = String(node.parameters.type ?? "simple");
  if (type === "random") {
    const sorted = [...items];
    for (let i = sorted.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [sorted[i], sorted[j]] = [sorted[j]!, sorted[i]!];
    }
    return { items: sorted };
  }
  if (type === "code") {
    const code = String(
      node.parameters.code ??
        "return a.json.value < b.json.value ? -1 : a.json.value > b.json.value ? 1 : 0;",
    );
    // eslint-disable-next-line no-new-func
    const cmp = new Function("a", "b", `"use strict"; ${code.includes("return") ? code : `return (${code});`}`);
    return { items: [...items].sort((a, b) => Number(cmp(a, b)) || 0) };
  }

  const sortFields =
    (node.parameters.sortFields as Array<{ field?: string; direction?: string }>) ??
    [];
  const fields =
    sortFields.length > 0
      ? sortFields
      : [
          {
            field: String(node.parameters.field ?? ""),
            direction: String(node.parameters.direction ?? "asc"),
          },
        ];

  const sorted = [...items].sort((a, b) => {
    for (const sf of fields) {
      const field = String(sf.field ?? "");
      const dir = String(sf.direction ?? "asc");
      const av = field ? getJsonPath(a, field) : undefined;
      const bv = field ? getJsonPath(b, field) : undefined;
      if (av == null && bv == null) continue;
      if (av == null) return 1;
      if (bv == null) return -1;
      const as = av as string | number;
      const bs = bv as string | number;
      if (as < bs) return dir === "desc" ? 1 : -1;
      if (as > bs) return dir === "desc" ? -1 : 1;
    }
    return 0;
  });
  return { items: sorted };
};

/** Limit first/last N. */
export const limitNode: NodeHandler = async ({ node, items }) => {
  const max = Math.max(0, Number(node.parameters.max ?? node.parameters.maxItems ?? 10));
  const keep = String(node.parameters.keep ?? "first");
  if (items.length <= max) return { items };
  if (keep === "last") return { items: items.slice(items.length - max) };
  return { items: items.slice(0, max) };
};

/**
 * SplitInBatches — real loop/done outputs with node context cursor.
 * First call (or reset): stash items, emit first batch on `loop`.
 * Subsequent calls while hasMore: emit next batch on `loop`.
 * When exhausted: emit on `done` and clear hasMore.
 */
export const splitInBatchesNode: NodeHandler = async ({
  node,
  items,
  getNodeContext,
  setNodeContext,
}) => {
  const size = Math.max(1, Number(node.parameters.batchSize ?? 10));
  const ctx = getNodeContext<{
    sourceItems?: WorkflowItem[];
    cursor?: number;
    hasMore?: boolean;
    initialized?: boolean;
  }>();

  let sourceItems = ctx.sourceItems;
  let cursor = ctx.cursor ?? 0;

  // Fresh run or first entry with new items
  if (!ctx.initialized || (items.length && cursor === 0 && !ctx.hasMore)) {
    sourceItems = items.length ? items : (sourceItems ?? []);
    cursor = 0;
    setNodeContext({
      sourceItems,
      cursor: 0,
      initialized: true,
      hasMore: sourceItems.length > 0,
    });
  }

  sourceItems = sourceItems ?? [];

  if (cursor >= sourceItems.length) {
    setNodeContext({
      sourceItems,
      cursor,
      hasMore: false,
      initialized: true,
    });
    return {
      items: [],
      outputs: { main: [], loop: [], done: sourceItems },
      continueLoop: false,
    };
  }

  const batch = sourceItems.slice(cursor, cursor + size);
  const nextCursor = cursor + size;
  const hasMore = nextCursor < sourceItems.length;
  setNodeContext({
    sourceItems,
    cursor: nextCursor,
    hasMore,
    initialized: true,
  });

  return {
    items: batch,
    outputs: {
      main: batch,
      loop: batch,
      done: hasMore ? [] : sourceItems,
    },
    continueLoop: hasMore,
  };
};

/**
 * Aggregate — individualFields (collect into arrays) or allItemData (one list).
 * Legacy numeric op still supported via operation=sum|avg|...
 */
export const aggregateNode: NodeHandler = async ({ node, items }) => {
  const mode = String(node.parameters.mode ?? "individualFields");
  const legacyOp = node.parameters.operation
    ? String(node.parameters.operation)
    : "";

  if (
    legacyOp &&
    ["count", "sum", "avg", "min", "max"].includes(legacyOp) &&
    !node.parameters.mode
  ) {
    const field = String(node.parameters.field ?? "value");
    const nums = items.map((i) => Number(getJsonPath(i, field) ?? 0));
    let value: number;
    switch (legacyOp) {
      case "sum":
        value = nums.reduce((a, b) => a + b, 0);
        break;
      case "avg":
        value = nums.length
          ? nums.reduce((a, b) => a + b, 0) / nums.length
          : 0;
        break;
      case "min":
        value = nums.length ? Math.min(...nums) : 0;
        break;
      case "max":
        value = nums.length ? Math.max(...nums) : 0;
        break;
      default:
        value = items.length;
    }
    return {
      items: [
        makeItem({
          operation: legacyOp,
          field,
          value,
          count: items.length,
        }),
      ],
    };
  }

  if (mode === "allItemData") {
    const dest = String(node.parameters.destinationFieldName ?? "data");
    return {
      items: [makeItem({ [dest]: items.map((i) => itemJson(i)) })],
    };
  }

  // individualFields
  const fields =
    (node.parameters.fieldsToAggregate as Array<{
      fieldToAggregate?: string;
      renameField?: boolean;
      outputFieldName?: string;
    }>) ?? [];
  const fieldList =
    fields.length > 0
      ? fields
      : [{ fieldToAggregate: String(node.parameters.field ?? "value") }];

  const out: Record<string, unknown> = {};
  for (const f of fieldList) {
    const path = String(f.fieldToAggregate ?? "value");
    const name =
      f.renameField && f.outputFieldName
        ? String(f.outputFieldName)
        : path.includes(".")
          ? path.split(".").pop()!
          : path;
    out[name] = items.map((i) => getJsonPath(i, path));
  }
  return { items: [makeItem(out)] };
};

export const itemListsNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "flatten");
  if (op === "flatten") {
    const field = String(node.parameters.field ?? "batch");
    const out: WorkflowItem[] = [];
    for (const item of items) {
      const arr = getJsonPath(item, field);
      if (Array.isArray(arr)) {
        for (const el of arr) {
          out.push(
            typeof el === "object" && el && !Array.isArray(el)
              ? toItem(el)
              : makeItem({ value: el as unknown }, item),
          );
        }
      } else {
        out.push(item);
      }
    }
    return { items: out };
  }
  return { items: [makeItem({ items: items.map((i) => itemJson(i)) })] };
};

/** Code — runOnceForAllItems | runOnceForEachItem with n8n-like helpers. */
export const codeNode: NodeHandler = async ({
  node,
  items,
  vars,
  expressionCtx,
}) => {
  const mode = String(node.parameters.mode ?? "runOnceForAllItems");
  const code = String(
    node.parameters.code ??
      "return items.map(i => ({ json: { ...i.json, ok: true } }));",
  );

  const $helpers = {
    httpRequest: async (opts: {
      url: string;
      method?: string;
      headers?: Record<string, string>;
      body?: unknown;
      qs?: Record<string, string | number | boolean | undefined>;
      json?: boolean;
    }) => {
      if (!opts?.url) throw new Error("$helpers.httpRequest requires url");
      let url = opts.url;
      if (opts.qs && Object.keys(opts.qs).length) {
        const u = new URL(url);
        for (const [k, v] of Object.entries(opts.qs)) {
          if (v === undefined) continue;
          u.searchParams.set(k, String(v));
        }
        url = u.toString();
      }
      const method = String(opts.method ?? "GET").toUpperCase();
      const headers: Record<string, string> = { ...(opts.headers ?? {}) };
      let body: BodyInit | undefined;
      if (opts.body !== undefined && method !== "GET" && method !== "HEAD") {
        if (
          typeof opts.body === "object" &&
          opts.body &&
          !(opts.body instanceof Buffer) &&
          typeof opts.body !== "string"
        ) {
          headers["content-type"] =
            headers["content-type"] ?? "application/json";
          body = JSON.stringify(opts.body);
        } else {
          body = opts.body as BodyInit;
        }
      }
      const res = await fetch(url, { method, headers, body });
      const text = await res.text();
      const wantJson = opts.json !== false;
      if (wantJson) {
        try {
          return JSON.parse(text);
        } catch {
          return text;
        }
      }
      return text;
    },
  };

  function nodeAccessor(name: string) {
    const entry =
      expressionCtx.runStore.get(name) ??
      [...expressionCtx.runStore.values()].find(
        (e) => e.nodeName === name || e.nodeId === name,
      );
    const main = entry?.outputs.main ?? [];
    return {
      item: main[0] ? itemJson(main[0]) : {},
      first: () => (main[0] ? itemJson(main[0]) : {}),
      last: () =>
        main.length ? itemJson(main[main.length - 1]!) : {},
      all: () => main.map((i) => itemJson(i)),
      json: main[0] ? itemJson(main[0]) : {},
      isExecuted: Boolean(entry),
    };
  }

  const $ = (name: string) => nodeAccessor(name);
  const $node: Record<string, ReturnType<typeof nodeAccessor>> = {};
  for (const entry of expressionCtx.runStore.values()) {
    $node[entry.nodeName] = nodeAccessor(entry.nodeName);
  }

  if (mode === "runOnceForEachItem") {
    const out: WorkflowItem[] = [];
    for (let i = 0; i < items.length; i++) {
      const item = items[i]!;
      const $json = itemJson(item);
      const $input = {
        item: $json,
        first: () => itemJson(items[0] ?? item),
        last: () => itemJson(items[items.length - 1] ?? item),
        all: () => items.map((x) => itemJson(x)),
      };
      // eslint-disable-next-line no-new-func
      const fn = new Function(
        "item",
        "items",
        "$json",
        "$itemIndex",
        "$input",
        "$vars",
        "$helpers",
        "$",
        "$node",
        `"use strict"; ${code.includes("return") ? code : `return (${code});`}`,
      );
      const result = fn(
        item,
        items,
        $json,
        i,
        $input,
        vars,
        $helpers,
        $,
        $node,
      );
      if (result == null) continue;
      if (Array.isArray(result)) {
        out.push(...ensureItems(result));
      } else {
        out.push(toItem(result, i));
      }
    }
    return { items: out };
  }

  const $input = {
    all: () => items,
    first: () => items[0],
    last: () => items[items.length - 1],
    item: items[0] ? itemJson(items[0]) : {},
  };
  const body = code.includes("return") ? code : `return (${code});`;
  // eslint-disable-next-line no-new-func
  const fn = new Function(
    "items",
    "$input",
    "$vars",
    "$helpers",
    "$",
    "$node",
    "cryptoLib",
    `"use strict"; return (async () => { ${body} })();`,
  );
  const result = await fn(
    items,
    $input,
    vars,
    $helpers,
    $,
    $node,
    { createHmac },
  );
  if (result == null) return { items: [] };
  return { items: ensureItems(Array.isArray(result) ? result : [result]) };
};

export const dateTimeNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "now");
  const now = new Date();
  if (op === "now") {
    return {
      items: [makeItem({ iso: now.toISOString(), epoch: now.getTime() })],
    };
  }
  const field = String(node.parameters.field ?? "date");
  return {
    items: items.map((item, idx) => {
      const raw = getJsonPath(item, field);
      const d = raw ? new Date(String(raw)) : now;
      return makeItem(
        {
          ...itemJson(item),
          [field + "_iso"]: d.toISOString(),
          [field + "_epoch"]: d.getTime(),
        },
        item,
        idx,
      );
    }),
  };
};

export const cryptoNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "hash");
  const algo = String(node.parameters.algorithm ?? "sha256");
  const field = String(node.parameters.field ?? "content");
  if (op === "random") {
    return {
      items: [
        makeItem({
          token: randomBytes(Number(node.parameters.bytes ?? 16)).toString("hex"),
        }),
      ],
    };
  }
  return {
    items: items.map((item, idx) => {
      const value = String(getJsonPath(item, field) ?? "");
      return makeItem(
        {
          ...itemJson(item),
          hash: createHash(algo).update(value).digest("hex"),
        },
        item,
        idx,
      );
    }),
  };
};

export const htmlNode: NodeHandler = async ({ node, items }) => {
  const template = String(
    node.parameters.template ?? "<pre>{{json}}</pre>",
  );
  return {
    items: items.map((item, idx) => {
      const json = itemJson(item);
      return makeItem(
        {
          html: template
            .replaceAll("{{json}}", JSON.stringify(json, null, 2))
            .replace(/\{\{(\w+)\}\}/g, (_, key: string) =>
              String(json[key] ?? ""),
            ),
        },
        item,
        idx,
      );
    }),
  };
};
