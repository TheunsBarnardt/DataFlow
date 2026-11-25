import { useMemo, useState } from "react";
import { AlignLeft, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { CodeEditor } from "@/components/CodeEditor";
import { formatCode } from "@/lib/formatCode";
import { cn } from "@/lib/utils";
import {
  SCHEMA_PROP_TYPE_OPTIONS,
  isEditableObjectSchema,
  isPlainSchemaObject,
  propertyRowsToSchema,
  schemaToPropertyRows,
  type SchemaPropType,
  type SchemaPropertyRow,
} from "@/lib/jsonSchemaSample";

type Mode = "fields" | "json";

function emptyRow(): SchemaPropertyRow {
  return { name: "", type: "string", example: "", required: false };
}

function parseSchemaValue(value: unknown): unknown {
  if (typeof value === "string") {
    const raw = value.trim();
    if (!raw) return {};
    try {
      return JSON.parse(raw) as unknown;
    } catch {
      return value;
    }
  }
  if (value === undefined || value === null) return {};
  return value;
}

export function JsonSchemaPropertiesEditor({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (next: unknown) => void;
}) {
  const parsed = useMemo(() => parseSchemaValue(value), [value]);
  const canEditFields = isEditableObjectSchema(parsed);
  const [mode, setMode] = useState<Mode>(canEditFields ? "fields" : "json");

  const rows: SchemaPropertyRow[] = useMemo(() => {
    const fromSchema = schemaToPropertyRows(parsed);
    // Postman-style: always keep a blank row ready to type into
    if (fromSchema.length === 0) return [emptyRow()];
    const last = fromSchema[fromSchema.length - 1];
    if (last && last.name.trim()) return [...fromSchema, emptyRow()];
    return fromSchema;
  }, [parsed]);

  const jsonText = useMemo(() => {
    if (typeof value === "string") return value;
    if (value === undefined || value === null) return "";
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return "";
    }
  }, [value]);

  function emitRows(nextRows: SchemaPropertyRow[]) {
    // Drop trailing blank rows before persisting
    const cleaned = nextRows.filter(
      (r, i) =>
        r.name.trim() ||
        r.example.trim() ||
        r.required ||
        i < nextRows.length - 1,
    );
    onChange(propertyRowsToSchema(cleaned.filter((r) => r.name.trim())));
  }

  function updateRow(index: number, patch: Partial<SchemaPropertyRow>) {
    const next = rows.map((r, i) => (i === index ? { ...r, ...patch } : r));
    // If editing the last blank row into a named param, append another blank
    const last = next[next.length - 1];
    if (last?.name.trim() && index === next.length - 1) {
      next.push(emptyRow());
    }
    emitRows(next);
  }

  function removeRow(index: number) {
    const next = rows.filter((_, i) => i !== index);
    emitRows(next.length ? next : [emptyRow()]);
  }

  function addRow() {
    const withoutTrailingBlank = rows.filter(
      (r, i) => i < rows.length - 1 || r.name.trim() || r.example.trim(),
    );
    emitRows([...withoutTrailingBlank, emptyRow(), emptyRow()]);
  }

  function switchToFields() {
    if (!canEditFields) {
      onChange(propertyRowsToSchema([]));
    }
    setMode("fields");
  }

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5">
          <button
            type="button"
            className={cn(
              "rounded px-2 py-0.5 text-[10px] font-medium",
              mode === "fields"
                ? "bg-card text-foreground shadow-sm"
                : "text-muted-foreground",
            )}
            onClick={switchToFields}
          >
            Params
          </button>
          <button
            type="button"
            className={cn(
              "rounded px-2 py-0.5 text-[10px] font-medium",
              mode === "json"
                ? "bg-card text-foreground shadow-sm"
                : "text-muted-foreground",
            )}
            onClick={() => setMode("json")}
          >
            JSON
          </button>
        </div>
        {mode === "json" ? (
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
            title="Format"
            onClick={() => {
              const result = formatCode(jsonText, "json");
              if (!result.ok) {
                toast.error(result.error);
                return;
              }
              try {
                onChange(JSON.parse(result.text) as unknown);
              } catch {
                onChange(result.text);
              }
            }}
          >
            <AlignLeft className="size-3" />
            Format
          </Button>
        ) : null}
      </div>

      {mode === "fields" ? (
        <>
          {!canEditFields && isPlainSchemaObject(parsed) && "$ref" in parsed ? (
            <p className="text-[10px] text-muted-foreground">
              Replaced $ref schema with editable properties. Switch back to JSON
              to restore a $ref if needed.
            </p>
          ) : null}
          <div className="overflow-hidden rounded-md border border-border">
            <div className="grid grid-cols-[minmax(0,1.2fr)_72px_minmax(0,1.4fr)_36px_28px] gap-1 border-b bg-muted/40 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
              <span>Key</span>
              <span>Type</span>
              <span>Value</span>
              <span className="text-center" title="Required">
                Req
              </span>
              <span />
            </div>
            <div className="flex flex-col">
              {rows.map((row, i) => {
                const isPlaceholder =
                  i === rows.length - 1 && !row.name.trim() && !row.example.trim();
                return (
                  <div
                    key={i}
                    className={cn(
                      "grid grid-cols-[minmax(0,1.2fr)_72px_minmax(0,1.4fr)_36px_28px] items-center gap-1 border-b border-border/60 px-1.5 py-1 last:border-b-0",
                      isPlaceholder && "bg-muted/10",
                    )}
                  >
                    <Input
                      className="h-8 min-w-0 border-0 bg-transparent px-1.5 text-xs shadow-none focus-visible:ring-0"
                      placeholder="Key"
                      value={row.name}
                      onChange={(e) => updateRow(i, { name: e.target.value })}
                    />
                    <FormSelect
                      className="h-8 border-0 bg-transparent px-0.5 text-[11px] shadow-none"
                      value={row.type}
                      options={SCHEMA_PROP_TYPE_OPTIONS}
                      onChange={(next) =>
                        updateRow(i, { type: next as SchemaPropType })
                      }
                    />
                    <Input
                      className="h-8 min-w-0 border-0 bg-transparent px-1.5 text-xs shadow-none focus-visible:ring-0"
                      placeholder="Value"
                      value={row.example}
                      onChange={(e) =>
                        updateRow(i, { example: e.target.value })
                      }
                    />
                    <div className="flex justify-center">
                      <Checkbox
                        checked={row.required}
                        onCheckedChange={(checked) =>
                          updateRow(i, { required: checked === true })
                        }
                        title="Required"
                        aria-label="Required"
                      />
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-xs"
                      className={cn(
                        "text-muted-foreground",
                        isPlaceholder && "invisible",
                      )}
                      onClick={() => removeRow(i)}
                      aria-label="Remove parameter"
                      disabled={isPlaceholder}
                    >
                      <Trash2 />
                    </Button>
                  </div>
                );
              })}
            </div>
          </div>
          <Button
            type="button"
            variant="outline"
            size="xs"
            className="self-start"
            onClick={addRow}
          >
            <Plus />
            Add parameter
          </Button>
          <p className="text-[10px] text-muted-foreground">
            Like Postman: set Key + Value for sample request data. Downstream
            nodes map them as <code className="text-[10px]">$json.query.*</code>{" "}
            / <code className="text-[10px]">$json.body.*</code>.
          </p>
        </>
      ) : (
        <CodeEditor
          value={jsonText}
          language="json"
          height={120}
          placeholder='{ "type": "object", "properties": { "username": { "type": "string", "example": "jdoe" } } }'
          onChange={(next) => {
            const raw = next.trim();
            if (!raw) {
              onChange({});
              return;
            }
            try {
              onChange(JSON.parse(raw) as unknown);
            } catch {
              onChange(next);
            }
          }}
        />
      )}
    </div>
  );
}
