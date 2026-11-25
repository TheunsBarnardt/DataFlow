import { variables } from "../db.js";

/** Load project variables as a flat key → value map. */
export async function loadVarMap(): Promise<Record<string, string>> {
  const list = await variables().find({}).toArray();
  const map: Record<string, string> = {};
  for (const v of list) {
    map[v.key] = v.value;
  }
  return map;
}

/**
 * Replace `$vars.KEY` (and `$vars['KEY']` / `$vars["KEY"]`) in a string.
 * Unknown keys are left unchanged.
 */
export function resolveVarsInString(
  input: string,
  map: Record<string, string>,
): string {
  return input
    .replace(/\$vars\.([A-Za-z_][A-Za-z0-9_]*)/g, (full, key: string) =>
      Object.prototype.hasOwnProperty.call(map, key) ? map[key]! : full,
    )
    .replace(/\$vars\[['"]([^'"]+)['"]\]/g, (full, key: string) =>
      Object.prototype.hasOwnProperty.call(map, key) ? map[key]! : full,
    );
}

/** Deep-walk parameters and resolve `$vars.*` in every string leaf. */
export function resolveVarsInValue<T>(value: T, map: Record<string, string>): T {
  if (typeof value === "string") {
    return resolveVarsInString(value, map) as T;
  }
  if (Array.isArray(value)) {
    return value.map((v) => resolveVarsInValue(v, map)) as T;
  }
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = resolveVarsInValue(v, map);
    }
    return out as T;
  }
  return value;
}
