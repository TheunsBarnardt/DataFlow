/**
 * Shared n8n-style condition engine for If / Filter / Switch.
 */

import { getJsonPath, type WorkflowItem } from "./items.js";

export type ConditionOp =
  | "equals"
  | "eq"
  | "notEquals"
  | "neq"
  | "contains"
  | "notContains"
  | "startsWith"
  | "endsWith"
  | "regex"
  | "exists"
  | "notExists"
  | "empty"
  | "notEmpty"
  | "truthy"
  | "true"
  | "falsy"
  | "false"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "isEmpty"
  | "isNotEmpty";

export type Condition = {
  /** Field path on $json, or leftValue when set */
  field?: string;
  leftValue?: unknown;
  operator?: string;
  value?: unknown;
  rightValue?: unknown;
  typeValidation?: "strict" | "loose";
};

export type ConditionGroup = {
  combinator?: "AND" | "OR" | "and" | "or";
  conditions?: Condition[];
  ignoreCase?: boolean;
};

function asString(v: unknown, ignoreCase: boolean): string {
  const s = String(v ?? "");
  return ignoreCase ? s.toLowerCase() : s;
}

export function compareValues(
  actual: unknown,
  op: string,
  expected: unknown,
  opts?: { ignoreCase?: boolean },
): boolean {
  const ignoreCase = Boolean(opts?.ignoreCase);
  const aStr = asString(actual, ignoreCase);
  const eStr = asString(expected, ignoreCase);

  switch (op) {
    case "equals":
    case "eq":
      if (ignoreCase && (typeof actual === "string" || typeof expected === "string")) {
        return aStr === eStr;
      }
      return actual == expected;
    case "notEquals":
    case "neq":
      if (ignoreCase && (typeof actual === "string" || typeof expected === "string")) {
        return aStr !== eStr;
      }
      return actual != expected;
    case "contains":
      return aStr.includes(eStr);
    case "notContains":
      return !aStr.includes(eStr);
    case "startsWith":
      return aStr.startsWith(eStr);
    case "endsWith":
      return aStr.endsWith(eStr);
    case "regex":
      try {
        return new RegExp(String(expected ?? ""), ignoreCase ? "i" : "").test(
          String(actual ?? ""),
        );
      } catch {
        return false;
      }
    case "exists":
      return actual !== undefined && actual !== null;
    case "notExists":
      return actual === undefined || actual === null;
    case "empty":
    case "isEmpty":
      if (actual === undefined || actual === null) return true;
      if (typeof actual === "string") return actual.length === 0;
      if (Array.isArray(actual)) return actual.length === 0;
      if (typeof actual === "object") return Object.keys(actual).length === 0;
      return false;
    case "notEmpty":
    case "isNotEmpty":
      return !compareValues(actual, "isEmpty", expected, opts);
    case "truthy":
    case "true":
      return Boolean(actual);
    case "falsy":
    case "false":
      return !actual;
    case "gt":
      return Number(actual) > Number(expected);
    case "gte":
      return Number(actual) >= Number(expected);
    case "lt":
      return Number(actual) < Number(expected);
    case "lte":
      return Number(actual) <= Number(expected);
    default:
      return Boolean(actual);
  }
}

export function leftOf(item: WorkflowItem, c: Condition): unknown {
  if (c.leftValue !== undefined) return c.leftValue;
  if (c.field) return getJsonPath(item, String(c.field));
  return undefined;
}

export function rightOf(c: Condition): unknown {
  return c.rightValue !== undefined ? c.rightValue : c.value;
}

export function matchConditions(
  item: WorkflowItem,
  conditions: Condition[],
  combinator: string,
  opts?: { ignoreCase?: boolean },
): boolean {
  if (!conditions.length) return true;
  const results = conditions.map((c) =>
    compareValues(
      leftOf(item, c),
      String(c.operator ?? "equals"),
      rightOf(c),
      opts,
    ),
  );
  const upper = combinator.toUpperCase();
  return upper === "OR" ? results.some(Boolean) : results.every(Boolean);
}

export function matchConditionGroup(
  item: WorkflowItem,
  group: ConditionGroup | undefined,
  legacy?: {
    field?: unknown;
    operator?: unknown;
    value?: unknown;
    conditions?: Condition[];
    combinator?: unknown;
  },
): boolean {
  let conditions = group?.conditions ?? legacy?.conditions ?? [];
  if (!conditions.length && legacy?.field) {
    conditions = [
      {
        field: String(legacy.field),
        operator: String(legacy.operator ?? "equals"),
        value: legacy.value,
      },
    ];
  }
  const combinator = String(
    group?.combinator ?? legacy?.combinator ?? "AND",
  );
  return matchConditions(item, conditions, combinator, {
    ignoreCase: group?.ignoreCase,
  });
}
