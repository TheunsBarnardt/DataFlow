import type { NodeHandler } from "../types.js";
import {
  ensureItems,
  getJsonPath,
  itemJson,
  makeItem,
  normalizeItems,
  type WorkflowItem,
} from "../items.js";
import {
  matchConditionGroup,
  matchConditions,
  type Condition,
  type ConditionGroup,
} from "../conditions.js";

/** Route each item to true or false using V2-style condition groups. */
export const ifNode: NodeHandler = async ({ node, items, resolve }) => {
  const group = (node.parameters.conditions as ConditionGroup) ?? undefined;
  const ignoreCase = Boolean(
    group?.ignoreCase ?? node.parameters.ignoreCase,
  );
  const truthy: WorkflowItem[] = [];
  const falsy: WorkflowItem[] = [];

  for (const item of items) {
    const resolvedGroup: ConditionGroup | undefined = group
      ? {
          ...group,
          conditions: (group.conditions ?? []).map((c) => {
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
          ignoreCase,
        }
      : undefined;

    const ok = matchConditionGroup(item, resolvedGroup, {
      field: node.parameters.field,
      operator: node.parameters.operator,
      value: resolve(node.parameters.value, item),
      conditions: node.parameters.conditions as Condition[] | undefined,
      combinator: node.parameters.combinator,
    });
    if (ok) truthy.push(item);
    else falsy.push(item);
  }

  return {
    items: truthy,
    outputs: { main: truthy, true: truthy, false: falsy },
    branch: truthy.length ? "true" : "false",
  };
};

/**
 * Switch — rules mode (conditions per output) or value mode (field vs cases).
 */
export const switchNode: NodeHandler = async ({ node, items, resolve }) => {
  const mode = String(node.parameters.mode ?? "rules");
  const fallbackOutput = String(node.parameters.fallbackOutput ?? "default");

  if (mode === "expression") {
    const outputs: Record<string, WorkflowItem[]> = { [fallbackOutput]: [] };
    for (const item of items) {
      const idx = resolve(node.parameters.output, item);
      const key = String(idx ?? fallbackOutput);
      if (!outputs[key]) outputs[key] = [];
      outputs[key]!.push(item);
    }
    const primaryKey =
      Object.keys(outputs).find((k) => (outputs[k]?.length ?? 0) > 0) ??
      fallbackOutput;
    return {
      items: outputs[primaryKey] ?? [],
      outputs: { main: outputs[primaryKey] ?? [], ...outputs },
      branch: primaryKey,
    };
  }

  if (mode === "rules") {
    const rules =
      (node.parameters.rules as Array<{
        outputKey?: string;
        conditions?: ConditionGroup | Condition[];
        combinator?: string;
      }>) ?? [];

    // Fallback to value/cases if no rules
    if (!rules.length && node.parameters.cases) {
      return switchValueMode(node.parameters, items, fallbackOutput);
    }

    const outputs: Record<string, WorkflowItem[]> = {
      [fallbackOutput]: [],
    };
    const sendToAll = Boolean(node.parameters.allMatchingOutputs);

    for (const item of items) {
      let matched = false;
      for (let i = 0; i < rules.length; i++) {
        const rule = rules[i]!;
        const key = String(rule.outputKey ?? `output${i}`);
        if (!outputs[key]) outputs[key] = [];
        const conds = Array.isArray(rule.conditions)
          ? (rule.conditions as Condition[])
          : ((rule.conditions as ConditionGroup)?.conditions ?? []);
        const combinator = String(
          (rule.conditions as ConditionGroup)?.combinator ??
            rule.combinator ??
            "AND",
        );
        const resolved = conds.map((c) => {
          const fieldStr = c.field != null ? String(c.field) : "";
          const fieldIsExpr = /\{\{/.test(fieldStr) || fieldStr.startsWith("=");
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
        });
        if (matchConditions(item, resolved, combinator)) {
          outputs[key]!.push(item);
          matched = true;
          if (!sendToAll) break;
        }
      }
      if (!matched) {
        outputs[fallbackOutput]!.push(item);
      }
    }

    const primaryKey =
      Object.keys(outputs).find((k) => (outputs[k]?.length ?? 0) > 0) ??
      fallbackOutput;
    return {
      items: outputs[primaryKey] ?? [],
      outputs: { main: outputs[primaryKey] ?? [], ...outputs },
      branch: primaryKey,
    };
  }

  // value mode (legacy cases)
  return switchValueMode(node.parameters, items, fallbackOutput);
};

function switchValueMode(
  parameters: Record<string, unknown>,
  items: WorkflowItem[],
  fallbackOutput: string,
): {
  items: WorkflowItem[];
  outputs: Record<string, WorkflowItem[]>;
  branch: string;
} {
  const field = String(parameters.field ?? "value");
  const cases = (parameters.cases as string[]) ?? [];
  const outputs: Record<string, WorkflowItem[]> = {
    [fallbackOutput]: [],
  };
  for (const c of cases) outputs[c] = [];

  for (const item of items) {
    const actual = String(getJsonPath(item, field) ?? "");
    const match = cases.find((c) => c === actual) ?? fallbackOutput;
    if (!outputs[match]) outputs[match] = [];
    outputs[match]!.push(item);
  }

  const primaryKey =
    cases.find((c) => (outputs[c]?.length ?? 0) > 0) ?? fallbackOutput;
  const primary = outputs[primaryKey] ?? [];
  return {
    items: primary,
    outputs: { main: primary, ...outputs },
    branch: primaryKey,
  };
}

function mergeInputs(
  inputsByHandle: Record<string, WorkflowItem[]>,
  items: WorkflowItem[],
): [WorkflowItem[], WorkflowItem[]] {
  const a =
    inputsByHandle["0"] ??
    inputsByHandle.input1 ??
    inputsByHandle.main ??
    items;
  const b = inputsByHandle["1"] ?? inputsByHandle.input2 ?? [];
  return [a, b];
}

/**
 * Merge: append | combineByFields | combineByPosition | combineAll | chooseBranch
 */
export const mergeNode: NodeHandler = async ({
  node,
  items,
  inputsByHandle,
}) => {
  const mode = String(node.parameters.mode ?? "append");
  const [input1, input2] = mergeInputs(inputsByHandle, items);

  if (mode === "chooseBranch") {
    const use = String(node.parameters.chooseBranch ?? "input1");
    return { items: use === "input2" ? input2 : input1 };
  }

  if (mode === "combineAll") {
    const out: WorkflowItem[] = [];
    for (const left of input1) {
      for (const right of input2) {
        out.push(
          makeItem({ ...itemJson(left), ...itemJson(right) }, left),
        );
      }
    }
    if (!input1.length) return { items: input2 };
    if (!input2.length) return { items: input1 };
    return { items: out };
  }

  if (mode === "combineByPosition") {
    const len = Math.max(input1.length, input2.length);
    const out: WorkflowItem[] = [];
    for (let i = 0; i < len; i++) {
      const left = input1[i];
      const right = input2[i];
      out.push(
        makeItem(
          {
            ...(left ? itemJson(left) : {}),
            ...(right ? itemJson(right) : {}),
          },
          left ?? right,
          i,
        ),
      );
    }
    return { items: out };
  }

  if (mode === "combineByFields") {
    // n8n join names + legacy aliases
    let joinMode = String(node.parameters.joinMode ?? "keepMatches");
    const alias: Record<string, string> = {
      inner: "keepMatches",
      left: "enrichInput1",
      keepEverything: "keepEverything",
      keepNonMatches: "keepNonMatches",
      enrichInput1: "enrichInput1",
      enrichInput2: "enrichInput2",
      keepMatches: "keepMatches",
    };
    joinMode = alias[joinMode] ?? joinMode;

    const fields1 = String(node.parameters.fields1 ?? node.parameters.field ?? "id")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const fields2 = String(node.parameters.fields2 ?? node.parameters.field ?? "id")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const keyOf = (item: WorkflowItem, fields: string[]) =>
      JSON.stringify(fields.map((f) => getJsonPath(item, f)));

    const map2 = new Map<string, WorkflowItem[]>();
    for (const item of input2) {
      const k = keyOf(item, fields2.length ? fields2 : fields1);
      if (!map2.has(k)) map2.set(k, []);
      map2.get(k)!.push(item);
    }

    const out: WorkflowItem[] = [];
    const matchedKeys = new Set<string>();

    for (const left of input1) {
      const k = keyOf(left, fields1);
      const rights = map2.get(k);
      if (rights?.length) {
        matchedKeys.add(k);
        if (joinMode === "keepNonMatches") continue;
        for (const right of rights) {
          if (joinMode === "enrichInput2") {
            out.push(makeItem({ ...itemJson(right), ...itemJson(left) }, right));
          } else {
            out.push(makeItem({ ...itemJson(left), ...itemJson(right) }, left));
          }
        }
      } else if (
        joinMode === "enrichInput1" ||
        joinMode === "keepEverything" ||
        joinMode === "keepNonMatches"
      ) {
        out.push(left);
      }
    }

    if (joinMode === "keepEverything" || joinMode === "keepNonMatches") {
      for (const [k, rights] of map2) {
        if (matchedKeys.has(k) && joinMode !== "keepNonMatches") continue;
        if (matchedKeys.has(k) && joinMode === "keepNonMatches") continue;
        if (!matchedKeys.has(k)) out.push(...rights);
      }
    }
    if (joinMode === "enrichInput2") {
      for (const [k, rights] of map2) {
        if (matchedKeys.has(k)) continue;
        out.push(...rights);
      }
    }

    return { items: out };
  }

  // append
  return { items: [...input1, ...input2] };
};

export const waitNode: NodeHandler = async ({ node, items }) => {
  const mode = String(node.parameters.resume ?? node.parameters.mode ?? "timeAmount");
  const unit = String(node.parameters.unit ?? "seconds");
  let amount = Number(node.parameters.amount ?? node.parameters.ms ?? 1);
  if (node.parameters.ms != null && node.parameters.amount == null) {
    amount = Number(node.parameters.ms);
  }

  const toMs = (): number => {
    if (node.parameters.ms != null && mode === "timeAmount" && !node.parameters.amount) {
      return Number(node.parameters.ms);
    }
    switch (unit) {
      case "milliseconds":
      case "ms":
        return amount;
      case "minutes":
        return amount * 60_000;
      case "hours":
        return amount * 3_600_000;
      case "seconds":
      default:
        return amount * 1000;
    }
  };

  const MAX_INLINE_MS = 15 * 60_000;

  if (mode === "webhook") {
    const { createResumeToken } = await import("../waitResume.js");
    return {
      items,
      wait: {
        resumeToken: createResumeToken(),
        continueFromNodeId: node.id,
      },
    };
  }

  if (mode === "specificTime" || mode === "dateTime") {
    const when = String(node.parameters.dateTime ?? node.parameters.resumeAt ?? "");
    const at = new Date(when);
    if (Number.isNaN(at.getTime())) {
      throw new Error("Wait: invalid dateTime");
    }
    const ms = at.getTime() - Date.now();
    if (ms <= 0) return { items };
    if (ms <= MAX_INLINE_MS) {
      await new Promise((r) => setTimeout(r, ms));
      return { items };
    }
    const { createResumeToken } = await import("../waitResume.js");
    return {
      items,
      wait: {
        resumeAt: at.toISOString(),
        resumeToken: createResumeToken(),
        continueFromNodeId: node.id,
      },
    };
  }

  // timeAmount (default)
  const ms = Math.max(0, toMs());
  if (ms <= MAX_INLINE_MS) {
    await new Promise((r) => setTimeout(r, ms));
    return { items };
  }
  const { createResumeToken } = await import("../waitResume.js");
  return {
    items,
    wait: {
      resumeAt: new Date(Date.now() + ms).toISOString(),
      resumeToken: createResumeToken(),
      continueFromNodeId: node.id,
    },
  };
};

export const manualNode: NodeHandler = async ({ items, runInput }) => ({
  items: items.length
    ? items
    : normalizeItems(runInput ?? { triggered: true }),
});

export const scheduleTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: items.length
    ? items
    : normalizeItems(runInput ?? { scheduled: true }),
});

export const webhookTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: items.length
    ? items
    : normalizeItems(
        runInput && typeof runInput === "object"
          ? runInput
          : { body: runInput },
      ),
});

/** API project endpoint trigger — same shape as webhook, keyed for OpenAPI sync. */
export const apiEndpointTriggerNode: NodeHandler = async ({
  items,
  runInput,
}) => ({
  items: items.length
    ? items
    : normalizeItems(
        runInput && typeof runInput === "object"
          ? runInput
          : { body: runInput },
      ),
});

export const mcpCallNode: NodeHandler = async ({ node, items, resolve }) => {
  const tool = String(node.parameters.tool ?? "");
  if (!tool) throw new Error("MCP Call: tool name is required");
  const baseArgs = (node.parameters.arguments as Record<string, unknown>) ?? {};
  const endpoint = String(node.parameters.endpoint ?? "").trim();

  // Optional remote MCP-style HTTP bridge: POST { tool, arguments } → JSON
  if (endpoint) {
    const first = items[0] ?? makeItem({});
    const args = {
      ...baseArgs,
      ...(items.length === 1 ? itemJson(first) : { items: items.map(itemJson) }),
    };
    const res = await fetch(String(resolve(endpoint, first)), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ tool, arguments: args }),
    });
    const text = await res.text();
    let data: unknown = text;
    try {
      data = JSON.parse(text);
    } catch {
      /* keep */
    }
    if (!res.ok) {
      throw new Error(
        typeof data === "object" && data && "error" in data
          ? String((data as { error: unknown }).error)
          : `MCP HTTP ${res.status}`,
      );
    }
    if (Array.isArray(data)) return { items: ensureItems(data) };
    return {
      items: [
        makeItem(
          typeof data === "object" && data
            ? (data as Record<string, unknown>)
            : { result: data },
        ),
      ],
    };
  }

  // Local agent tools (same surface as chat/MCP stdio)
  const { callAgentTool } = await import("../../agent/tools.js");
  const out: ReturnType<typeof makeItem>[] = [];
  for (const item of items.length ? items : [makeItem({})]) {
    const args = { ...baseArgs, ...itemJson(item) };
    const result = await callAgentTool(tool, args);
    if (!result.ok) {
      throw new Error(result.error ?? `MCP tool "${tool}" failed`);
    }
    const data = result.data;
    if (Array.isArray(data)) {
      out.push(...ensureItems(data).map((x) => makeItem(itemJson(x), item)));
    } else if (data && typeof data === "object") {
      out.push(makeItem(data as Record<string, unknown>, item));
    } else {
      out.push(makeItem({ result: data, tool }, item));
    }
  }
  return { items: out };
};
