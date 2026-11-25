import { useMemo } from "react";
import { Pin } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

/** Flatten nested JSON into clickable `$json` expression paths. */
function flattenPaths(
  value: unknown,
  prefix = "$json",
  out: { path: string; preview: string }[] = [],
  depth = 0,
): { path: string; preview: string }[] {
  if (depth > 4) return out;
  if (value === null || value === undefined) {
    out.push({ path: `{{ ${prefix} }}`, preview: String(value) });
    return out;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) {
      out.push({ path: `{{ ${prefix} }}`, preview: "[]" });
      return out;
    }
    // Prefer first item fields for mapper UX
    return flattenPaths(value[0], `${prefix}[0]`, out, depth + 1);
  }
  if (typeof value === "object") {
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      const next = `${prefix}.${k}`;
      if (v !== null && typeof v === "object") {
        flattenPaths(v, next, out, depth + 1);
      } else {
        out.push({
          path: `{{ ${next} }}`,
          preview: typeof v === "string" ? v : JSON.stringify(v),
        });
      }
    }
    return out;
  }
  out.push({ path: `{{ ${prefix} }}`, preview: String(value) });
  return out;
}

export function NodeDataMapper({
  title,
  data,
  emptyHint,
  onPinData,
}: {
  title: string;
  data: unknown;
  emptyHint: string;
  /** Pin current output as sample data for downstream mapping / execute-step input. */
  onPinData?: () => void;
}) {
  const fields = useMemo(() => {
    if (data == null) return [];
    // If array of items, map first item's json-ish shape
    if (Array.isArray(data) && data.length) {
      const first = data[0];
      if (first && typeof first === "object" && "json" in (first as object)) {
        return flattenPaths((first as { json: unknown }).json, "$json");
      }
      return flattenPaths(first, "$json");
    }
    return flattenPaths(data, "$json");
  }, [data]);

  async function copyExpr(expr: string) {
    try {
      await navigator.clipboard.writeText(expr);
      toast.success("Expression copied", { description: expr });
    } catch {
      toast.message(expr);
    }
  }

  return (
    <div className="rounded-md border bg-muted/20 p-2">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
          {title}
        </div>
        {onPinData && data != null ? (
          <Button
            type="button"
            size="xs"
            variant="outline"
            className="h-6 gap-1 px-1.5 text-[10px]"
            title="Pin this output as sample data"
            onClick={() => {
              onPinData();
              toast.success("Data pinned");
            }}
          >
            <Pin className="size-3" />
            Pin data
          </Button>
        ) : null}
      </div>
      {!fields.length ? (
        <p className="text-[11px] text-muted-foreground">{emptyHint}</p>
      ) : (
        <ul className="flex max-h-40 flex-col gap-0.5 overflow-auto">
          {fields.slice(0, 40).map((f) => (
            <li key={f.path}>
              <button
                type="button"
                className="flex w-full items-center justify-between gap-2 rounded px-1.5 py-1 text-left hover:bg-muted"
                title="Copy expression"
                onClick={() => void copyExpr(f.path)}
              >
                <code className="truncate font-mono text-[10px] text-foreground">
                  {f.path}
                </code>
                <span className="max-w-[40%] truncate text-[10px] text-muted-foreground">
                  {f.preview}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
