/** n8n-style workflow item: data lives under `json`, optional binary + pairing. */

export type WorkflowItem = {
  json: Record<string, unknown>;
  binary?: Record<string, unknown>;
  pairedItem?: { item: number } | Array<{ item: number }>;
};

/** @deprecated Prefer WorkflowItem; kept as alias for gradual migration. */
export type Item = WorkflowItem;

function isWorkflowItem(value: unknown): value is WorkflowItem {
  return (
    !!value &&
    typeof value === "object" &&
    "json" in value &&
    typeof (value as WorkflowItem).json === "object" &&
    (value as WorkflowItem).json !== null &&
    !Array.isArray((value as WorkflowItem).json)
  );
}

/** True when obj is only a workflow envelope (not a data row that happens to have `json`). */
function isEnvelopeOnly(value: unknown): value is WorkflowItem {
  if (!isWorkflowItem(value)) return false;
  return Object.keys(value).every((k) =>
    ["json", "pairedItem", "binary"].includes(k),
  );
}

/**
 * Collapse accidental nested envelopes: `{ json: { json: row } }` → payload `row`.
 * Keeps outermost pairedItem/binary when present.
 */
function unwrapPayload(raw: unknown): {
  json: Record<string, unknown>;
  binary?: Record<string, unknown>;
  pairedItem?: WorkflowItem["pairedItem"];
} {
  let cur: unknown = raw;
  let binary: Record<string, unknown> | undefined;
  let pairedItem: WorkflowItem["pairedItem"] | undefined;

  for (let i = 0; i < 4; i++) {
    if (!isEnvelopeOnly(cur)) break;
    if (cur.binary && !binary) binary = { ...cur.binary };
    if (cur.pairedItem !== undefined && pairedItem === undefined) {
      pairedItem = cur.pairedItem;
    }
    cur = cur.json;
  }

  if (cur && typeof cur === "object" && !Array.isArray(cur)) {
    return {
      json: { ...(cur as Record<string, unknown>) },
      ...(binary ? { binary } : {}),
      ...(pairedItem !== undefined ? { pairedItem } : {}),
    };
  }
  return {
    json: { value: cur as unknown },
    ...(binary ? { binary } : {}),
    ...(pairedItem !== undefined ? { pairedItem } : {}),
  };
}

/** Convert a flat object or already-structured item into WorkflowItem. */
export function toItem(raw: unknown, pairedIndex?: number): WorkflowItem {
  if (raw == null) {
    return pairedIndex !== undefined
      ? { json: {}, pairedItem: { item: pairedIndex } }
      : { json: {} };
  }
  if (typeof raw === "object") {
    const unwrapped = unwrapPayload(raw);
    const item: WorkflowItem = { json: unwrapped.json };
    if (unwrapped.binary) item.binary = unwrapped.binary;
    if (unwrapped.pairedItem !== undefined) item.pairedItem = unwrapped.pairedItem;
    else if (pairedIndex !== undefined) item.pairedItem = { item: pairedIndex };
    return item;
  }
  const item: WorkflowItem = { json: { value: raw as unknown } };
  if (pairedIndex !== undefined) item.pairedItem = { item: pairedIndex };
  return item;
}

/** Normalize any input array / single value into WorkflowItem[]. */
export function normalizeItems(input: unknown): WorkflowItem[] {
  if (input == null) return [];
  if (Array.isArray(input)) {
    return input.map((el, i) => toItem(el, i));
  }
  return [toItem(input, 0)];
}

/** Ensure handler output is structured WorkflowItem[]. Accepts flat or structured. */
export function ensureItems(items: unknown): WorkflowItem[] {
  if (!Array.isArray(items)) {
    if (items == null) return [];
    return [toItem(items)];
  }
  return items.map((el, i) => toItem(el, i));
}

/** Read json payload (handles legacy flat items and accidental double-wraps). */
export function itemJson(item: WorkflowItem | Record<string, unknown>): Record<string, unknown> {
  if (isEnvelopeOnly(item) || isWorkflowItem(item)) {
    return unwrapPayload(item).json;
  }
  return item as Record<string, unknown>;
}

/** Build a new item from json, optionally copying binary/paired from a source. */
export function makeItem(
  json: Record<string, unknown>,
  source?: WorkflowItem,
  pairedIndex?: number,
): WorkflowItem {
  // If a caller passes a WorkflowItem as `json`, unwrap so we never nest
  // `{ json: { json, pairedItem } }` (historical mssql / ensureItems bug).
  const payload = isEnvelopeOnly(json) ? unwrapPayload(json).json : json;
  const item: WorkflowItem = { json: payload };
  if (source?.binary) item.binary = { ...source.binary };
  if (pairedIndex !== undefined) item.pairedItem = { item: pairedIndex };
  else if (source?.pairedItem !== undefined) item.pairedItem = source.pairedItem;
  return item;
}

/** Dot-path get on an item's json (or flat object). */
export function getJsonPath(
  item: WorkflowItem | Record<string, unknown>,
  path: string,
): unknown {
  const obj = itemJson(item);
  if (!path) return obj;
  return path.split(".").reduce<unknown>((acc, key) => {
    if (acc && typeof acc === "object") {
      return (acc as Record<string, unknown>)[key];
    }
    return undefined;
  }, obj);
}

/** Dot-path set on a plain object (mutates). */
export function setJsonPath(
  obj: Record<string, unknown>,
  path: string,
  value: unknown,
): void {
  if (!path) return;
  const parts = path.split(".");
  let cur: Record<string, unknown> = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i]!;
    const next = cur[key];
    if (!next || typeof next !== "object" || Array.isArray(next)) {
      cur[key] = {};
    }
    cur = cur[key] as Record<string, unknown>;
  }
  cur[parts[parts.length - 1]!] = value;
}

/** Collect minutesSaved from structured or flat log outputs. */
export function minutesFromItemLike(item: unknown): number {
  if (!item || typeof item !== "object") return 0;
  if (isWorkflowItem(item)) {
    return Number(item.json.minutesSaved) || 0;
  }
  return Number((item as { minutesSaved?: number }).minutesSaved) || 0;
}
