function usableText(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const t = value.trim();
  if (!t || t === "[object Object]") return undefined;
  return t;
}

/** Pull a human-readable message from thrown Errors, plain objects, or nested driver errors. */
export function errorMessage(err: unknown): string {
  if (err == null) return "Unknown error";
  if (typeof err === "string") return usableText(err) ?? err;

  if (Array.isArray(err)) {
    const parts = err.map(errorMessage).filter((m) => m && m !== "Unknown error");
    if (parts.length) return parts.join("; ");
  }

  if (err instanceof Error) {
    const msg = usableText(err.message);
    if (msg) return msg;

    const nested = (err as Error & { originalError?: unknown }).originalError;
    if (nested != null) {
      const fromNested = errorMessage(nested);
      if (fromNested && fromNested !== "Unknown error") return fromNested;
    }
  }

  if (typeof err === "object") {
    const o = err as Record<string, unknown>;
    // ODBC / msnodesqlv8 often puts the useful line in details[]
    if (Array.isArray(o.details) && o.details.length) {
      const fromDetails = errorMessage(o.details);
      if (fromDetails && fromDetails !== "Unknown error") return fromDetails;
    }
    for (const key of ["message", "sqlMessage", "Message", "error"] as const) {
      const v = o[key];
      const asText = usableText(v);
      if (asText) return asText;
      if (v && typeof v === "object") {
        const nested = errorMessage(v);
        if (nested && nested !== "Unknown error") return nested;
      }
    }
    const sqlstate = usableText(o.sqlstate) ?? usableText(o.sqlState);
    const code = o.code != null ? String(o.code) : undefined;
    try {
      const json = JSON.stringify(err);
      // Skip useless Error serializations like {"name":"ConnectionError"}
      if (
        json &&
        json !== "{}" &&
        json !== "null" &&
        !/^\{"name":"[^"]+"\}$/.test(json)
      ) {
        return sqlstate || code ? `${sqlstate ?? code}: ${json}` : json;
      }
    } catch {
      /* ignore */
    }
    if (sqlstate) return sqlstate;
  }

  const s = String(err);
  return s === "[object Object]" ? "Unknown error" : s;
}
