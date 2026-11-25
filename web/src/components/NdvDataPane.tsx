import { useMemo, useState } from "react";
import { Pin } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { CodeEditor } from "@/components/CodeEditor";
import { cn } from "@/lib/utils";
import { BinaryViewer, hasBinaryData } from "@/components/BinaryViewer";

export const DF_DRAG_MIME = "application/x-dataflow-expr";

function asItems(data: unknown): unknown[] {
  if (data == null) return [];
  return Array.isArray(data) ? data : [data];
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return !!v && typeof v === "object" && !Array.isArray(v);
}

function isEnvelope(item: unknown): item is Record<string, unknown> {
  return (
    isPlainObject(item) &&
    "json" in item &&
    isPlainObject(item.json) &&
    // Treat as envelope when it looks like a workflow item, not a data row
    // that happens to have a `json` field among others.
    ("pairedItem" in item ||
      "binary" in item ||
      Object.keys(item).every((k) =>
        ["json", "pairedItem", "binary"].includes(k),
      ))
  );
}

/** Unwrap n8n-style { json, pairedItem } and accidental double-wraps. */
function itemJson(item: unknown): Record<string, unknown> {
  let cur: unknown = item;
  for (let i = 0; i < 4; i++) {
    if (!isEnvelope(cur)) break;
    cur = cur.json;
  }
  if (isPlainObject(cur)) return cur;
  return { value: cur as unknown };
}

/** Lift pairedItem/binary from nested envelopes for the JSON tab. */
function envelopeMeta(item: unknown): {
  pairedItem?: unknown;
  binary?: unknown;
} {
  let cur: unknown = item;
  let pairedItem: unknown;
  let binary: unknown;
  for (let i = 0; i < 4; i++) {
    if (!isEnvelope(cur)) break;
    if (pairedItem === undefined && "pairedItem" in cur) pairedItem = cur.pairedItem;
    if (binary === undefined && "binary" in cur) binary = cur.binary;
    cur = cur.json;
  }
  return {
    ...(pairedItem !== undefined ? { pairedItem } : {}),
    ...(binary !== undefined ? { binary } : {}),
  };
}

/**
 * Table rows: one object per row. Expands `__raw` / `value` arrays of objects
 * so SQL/API list payloads show real columns (Id, Company, …).
 */
function toTableRows(items: Record<string, unknown>[]): Record<string, unknown>[] {
  if (!items.length) return [];

  // Single item wrapping a list of row objects
  if (items.length === 1) {
    const only = items[0]!;
    for (const key of ["__raw", "value", "rows", "data"] as const) {
      const v = only[key];
      if (
        Array.isArray(v) &&
        v.length > 0 &&
        v.every((el) => isPlainObject(el))
      ) {
        return v as Record<string, unknown>[];
      }
    }
  }

  return items.map((row) => {
    // Prefer expanding __raw object/array leftover on a row
    if ("__raw" in row && isPlainObject(row.__raw) && Object.keys(row).length <= 2) {
      return row.__raw as Record<string, unknown>;
    }
    return row;
  });
}

function typeOf(v: unknown): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return "array";
  return typeof v;
}

function flattenPaths(
  value: unknown,
  prefix = "$json",
  out: { path: string; expr: string; preview: string; type: string }[] = [],
  depth = 0,
): { path: string; expr: string; preview: string; type: string }[] {
  if (depth > 5) return out;
  if (value === null || value === undefined) {
    out.push({
      path: prefix,
      expr: `={{ ${prefix} }}`,
      preview: String(value),
      type: typeOf(value),
    });
    return out;
  }
  if (Array.isArray(value)) {
    if (!value.length) {
      out.push({
        path: prefix,
        expr: `={{ ${prefix} }}`,
        preview: "[]",
        type: "array",
      });
      return out;
    }
    return flattenPaths(value[0], `${prefix}[0]`, out, depth + 1);
  }
  if (typeof value === "object") {
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      const next = `${prefix}.${k}`;
      if (v !== null && typeof v === "object") {
        flattenPaths(v, next, out, depth + 1);
      } else {
        out.push({
          path: next,
          expr: `={{ ${next} }}`,
          preview: typeof v === "string" ? v : JSON.stringify(v),
          type: typeOf(v),
        });
      }
    }
    return out;
  }
  out.push({
    path: prefix,
    expr: `={{ ${prefix} }}`,
    preview: String(value),
    type: typeOf(value),
  });
  return out;
}

type View = "schema" | "table" | "json";

export function NdvDataPane({
  title,
  data,
  emptyHint,
  onPinData,
  exprPrefix,
}: {
  title: string;
  data: unknown;
  emptyHint: string;
  onPinData?: () => void;
  /** Override expression root, e.g. $('Node').item */
  exprPrefix?: string;
}) {
  const [view, setView] = useState<View>("schema");
  const rawItems = useMemo(() => asItems(data), [data]);
  const items = useMemo(() => rawItems.map(itemJson), [rawItems]);
  /** n8n-style list for JSON tab — envelopes collapsed, payload under `json`. */
  const jsonDisplay = useMemo(
    () =>
      rawItems.map((raw, i) => ({
        json: items[i] ?? {},
        ...envelopeMeta(raw),
      })),
    [rawItems, items],
  );
  const jsonText = useMemo(
    () => JSON.stringify(jsonDisplay, null, 2),
    [jsonDisplay],
  );
  const tableRows = useMemo(() => toTableRows(items), [items]);
  const columns = useMemo(() => {
    const keys = new Set<string>();
    for (const row of tableRows) {
      for (const k of Object.keys(row)) {
        if (k === "pairedItem" || k === "binary") continue;
        keys.add(k);
      }
    }
    return [...keys].slice(0, 24);
  }, [tableRows]);

  const schemaFields = useMemo(() => {
    const first = tableRows[0] ?? items[0];
    if (!first) return [];
    const root = exprPrefix ?? "$json";
    return flattenPaths(first, root);
  }, [items, tableRows, exprPrefix]);

  async function copyExpr(expr: string) {
    try {
      await navigator.clipboard.writeText(expr);
      toast.success("Expression copied", { description: expr });
    } catch {
      toast.message(expr);
    }
  }

  function onDragStart(e: React.DragEvent, expr: string) {
    e.dataTransfer.setData(DF_DRAG_MIME, expr);
    e.dataTransfer.setData("text/plain", expr);
    e.dataTransfer.effectAllowed = "copy";
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
          {title}
        </div>
        {onPinData && data != null ? (
          <Button
            type="button"
            size="xs"
            variant="outline"
            className="h-6 gap-1 px-1.5 text-[10px]"
            onClick={() => {
              onPinData();
              toast.success("Data pinned");
            }}
          >
            <Pin className="size-3" />
            Pin
          </Button>
        ) : null}
      </div>

      <div className="flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5">
        {(["schema", "table", "json"] as const).map((v) => (
          <button
            key={v}
            type="button"
            className={cn(
              "flex-1 rounded px-2 py-1 text-[10px] font-medium capitalize",
              view === v
                ? "bg-card text-foreground shadow-sm"
                : "text-muted-foreground",
            )}
            onClick={() => setView(v)}
          >
            {v}
          </button>
        ))}
      </div>

      {data == null ? (
        <p className="text-[11px] text-muted-foreground">{emptyHint}</p>
      ) : view === "json" ? (
        <div className="min-h-0 flex-1">
          <CodeEditor
            value={jsonText}
            language="json"
            height="100%"
            readOnly
            className="h-full min-h-0 focus-within:ring-0"
            onChange={() => {}}
          />
        </div>
      ) : view === "table" ? (
        <div className="min-h-0 flex-1 overflow-auto rounded-md border">
          {!tableRows.length ? (
            <p className="p-2 text-[11px] text-muted-foreground">{emptyHint}</p>
          ) : (
            <table className="w-full border-collapse text-left text-[10px]">
              <thead className="sticky top-0 bg-muted/80">
                <tr>
                  <th className="border-b px-2 py-1 font-medium">#</th>
                  {columns.map((c) => (
                    <th key={c} className="border-b px-2 py-1 font-medium">
                      <span
                        draggable
                        onDragStart={(e) =>
                          onDragStart(
                            e,
                            `={{ ${(exprPrefix ?? "$json")}.${c} }}`,
                          )
                        }
                        className="cursor-grab"
                        title="Drag into a parameter"
                      >
                        {c}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {tableRows.slice(0, 50).map((row, i) => (
                  <tr key={i} className="odd:bg-muted/20">
                    <td className="border-b px-2 py-1 text-muted-foreground">
                      {i}
                    </td>
                    {columns.map((c) => (
                      <td
                        key={c}
                        className="max-w-[10rem] truncate border-b px-2 py-1"
                        title={
                          row[c] === undefined
                            ? undefined
                            : typeof row[c] === "string"
                              ? String(row[c])
                              : JSON.stringify(row[c])
                        }
                      >
                        {row[c] === undefined
                          ? ""
                          : typeof row[c] === "string" ||
                              typeof row[c] === "number" ||
                              typeof row[c] === "boolean"
                            ? String(row[c])
                            : JSON.stringify(row[c])}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-auto rounded-md border bg-muted/20 p-1.5">
          {!schemaFields.length ? (
            <p className="px-1 text-[11px] text-muted-foreground">{emptyHint}</p>
          ) : (
            schemaFields.slice(0, 60).map((f) => (
              <li key={f.path}>
                <button
                  type="button"
                  draggable
                  onDragStart={(e) => onDragStart(e, f.expr)}
                  className="flex w-full cursor-grab items-center justify-between gap-2 rounded px-1.5 py-1 text-left hover:bg-muted active:cursor-grabbing"
                  title="Drag into a parameter or click to copy"
                  onClick={() => void copyExpr(f.expr)}
                >
                  <span className="min-w-0">
                    <code className="block truncate font-mono text-[10px]">
                      {f.path}
                    </code>
                    <span className="text-[9px] text-muted-foreground">
                      {f.type}
                    </span>
                  </span>
                  <span className="max-w-[35%] truncate text-[10px] text-muted-foreground">
                    {f.preview}
                  </span>
                </button>
              </li>
            ))
          )}
        </ul>
      )}

      {hasBinaryData(data) ? <BinaryViewer data={data} /> : null}
    </div>
  );
}
