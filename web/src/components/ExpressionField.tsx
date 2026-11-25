import { useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import {
  VariableAwareInput,
  VariableAwareTextarea,
} from "@/components/VariableAwareField";
import { DF_DRAG_MIME } from "@/components/NdvDataPane";

function isExpression(value: string): boolean {
  const t = value.trim();
  return t.startsWith("={{") || t.startsWith("{{");
}

function toExpression(value: string): string {
  if (isExpression(value)) return value;
  if (!value) return "={{  }}";
  return `={{ ${JSON.stringify(value)} }}`;
}

function fromExpression(value: string): string {
  const t = value.trim();
  const m = t.match(/^=\{\{\s*([\s\S]*?)\s*\}\}$/);
  if (!m) return value;
  const inner = m[1]!.trim();
  if (
    (inner.startsWith('"') && inner.endsWith('"')) ||
    (inner.startsWith("'") && inner.endsWith("'"))
  ) {
    try {
      return JSON.parse(
        inner.startsWith("'") ? `"${inner.slice(1, -1)}"` : inner,
      );
    } catch {
      return inner.slice(1, -1);
    }
  }
  return value;
}

/**
 * Fixed ↔ Expression toggle (n8n-style) wrapping VariableAware fields.
 * Accepts drag-and-drop of expressions from NdvDataPane.
 */
export function ExpressionField({
  id,
  value,
  onChange,
  placeholder,
  multiline,
  className,
}: {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  multiline?: boolean;
  className?: string;
}) {
  const [mode, setMode] = useState<"fixed" | "expression">(() =>
    isExpression(value) ? "expression" : "fixed",
  );
  const [dragOver, setDragOver] = useState(false);

  const suggestions = useMemo(
    () => ["$json.", "$input.item", "$vars.", "$('NodeName').item.", "$now"],
    [],
  );

  function switchMode(next: "fixed" | "expression") {
    if (next === mode) return;
    if (next === "expression") {
      onChange(toExpression(value));
    } else {
      onChange(fromExpression(value));
    }
    setMode(next);
  }

  function acceptDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragOver(false);
    const expr =
      e.dataTransfer.getData(DF_DRAG_MIME) ||
      e.dataTransfer.getData("text/plain");
    if (!expr) return;
    const normalized = expr.trim().startsWith("={{")
      ? expr.trim()
      : expr.trim().startsWith("{{")
        ? `=${expr.trim()}`
        : `={{ ${expr.trim()} }}`;
    setMode("expression");
    onChange(normalized);
  }

  return (
    <div
      className={cn(
        "flex flex-col gap-1 rounded-md",
        dragOver && "ring-2 ring-[var(--brand)]",
      )}
      onDragOver={(e) => {
        if (
          e.dataTransfer.types.includes(DF_DRAG_MIME) ||
          e.dataTransfer.types.includes("text/plain")
        ) {
          e.preventDefault();
          setDragOver(true);
        }
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={acceptDrop}
    >
      <div className="flex items-center gap-1">
        <button
          type="button"
          className={cn(
            "rounded px-1.5 py-0.5 text-[10px] font-medium",
            mode === "fixed"
              ? "bg-primary text-primary-foreground"
              : "bg-muted text-muted-foreground",
          )}
          onClick={() => switchMode("fixed")}
        >
          Fixed
        </button>
        <button
          type="button"
          className={cn(
            "rounded px-1.5 py-0.5 text-[10px] font-medium",
            mode === "expression"
              ? "bg-primary text-primary-foreground"
              : "bg-muted text-muted-foreground",
          )}
          onClick={() => switchMode("expression")}
        >
          Expression
        </button>
        {mode === "expression" ? (
          <div className="ml-auto flex flex-wrap gap-0.5">
            {suggestions.map((s) => (
              <button
                key={s}
                type="button"
                className="rounded border px-1 text-[9px] text-muted-foreground hover:bg-muted"
                title={`Insert ${s}`}
                onClick={() => {
                  const base = value.trim() || "={{  }}";
                  if (base.includes("}}")) {
                    onChange(base.replace(/\}\}\s*$/, `${s} }}`));
                  } else {
                    onChange(`={{ ${s} }}`);
                  }
                }}
              >
                {s}
              </button>
            ))}
          </div>
        ) : null}
      </div>
      {multiline ? (
        <VariableAwareTextarea
          id={id}
          className={cn("min-h-[72px] text-xs", className)}
          value={value}
          placeholder={
            mode === "expression"
              ? "={{ $json.field }} — or drop a field here"
              : placeholder
          }
          onChange={onChange}
        />
      ) : (
        <VariableAwareInput
          id={id}
          className={cn("h-8 text-xs", className)}
          value={value}
          placeholder={
            mode === "expression"
              ? "={{ $json.field }} — or drop a field here"
              : placeholder
          }
          onChange={onChange}
        />
      )}
    </div>
  );
}
