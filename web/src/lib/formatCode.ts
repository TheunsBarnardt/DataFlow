import { format as formatSql } from "sql-formatter";

const VARS_TOKEN_RE = /\$vars\.[A-Za-z_][A-Za-z0-9_]*/g;

/**
 * Protect `$vars.KEY` tokens so formatters don't mangle them,
 * then restore after formatting.
 */
function withProtectedVars(source: string, run: (src: string) => string): string {
  const tokens: string[] = [];
  const masked = source.replace(VARS_TOKEN_RE, (m) => {
    const i = tokens.length;
    tokens.push(m);
    return `__BF_VAR_${i}__`;
  });
  const formatted = run(masked);
  return formatted.replace(/__BF_VAR_(\d+)__/g, (_, n: string) => {
    return tokens[Number(n)] ?? _;
  });
}

export function formatSqlQuery(source: string): string {
  const trimmed = source.trim();
  if (!trimmed) return source;
  return withProtectedVars(source, (src) =>
    formatSql(src, {
      language: "tsql",
      tabWidth: 2,
      keywordCase: "upper",
      linesBetweenQueries: 1,
    }),
  );
}

export function formatJsonText(source: string): string | null {
  try {
    const parsed = JSON.parse(source) as unknown;
    return JSON.stringify(parsed, null, 2);
  } catch {
    return null;
  }
}

export type CodeLanguage = "sql" | "json" | "js" | "text";

export function formatCode(
  source: string,
  language: CodeLanguage,
): { ok: true; text: string } | { ok: false; error: string } {
  try {
    if (language === "sql") {
      return { ok: true, text: formatSqlQuery(source) };
    }
    if (language === "json") {
      const text = formatJsonText(source);
      if (text == null) return { ok: false, error: "Invalid JSON" };
      return { ok: true, text };
    }
    return { ok: true, text: source };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : "Format failed",
    };
  }
}
