import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { COMPARE_OPS } from "@/lib/nodeSchemas";
import { ExpressionField } from "@/components/ExpressionField";

export type ConditionRow = {
  field?: string;
  operator?: string;
  value?: unknown;
};

export type ConditionGroupValue = {
  combinator?: "AND" | "OR";
  conditions?: ConditionRow[];
  ignoreCase?: boolean;
};

function normalize(value: unknown): ConditionGroupValue {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    const g = value as ConditionGroupValue;
    return {
      combinator: (g.combinator ?? "AND") as "AND" | "OR",
      conditions: Array.isArray(g.conditions) ? g.conditions : [],
      ignoreCase: Boolean(g.ignoreCase),
    };
  }
  if (Array.isArray(value)) {
    return { combinator: "AND", conditions: value as ConditionRow[] };
  }
  return { combinator: "AND", conditions: [] };
}

export function ConditionBuilder({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (next: ConditionGroupValue) => void;
}) {
  const group = normalize(value);
  const rows = group.conditions ?? [];

  function updateRow(index: number, patch: Partial<ConditionRow>) {
    const next = rows.map((r, i) => (i === index ? { ...r, ...patch } : r));
    onChange({ ...group, conditions: next });
  }

  function removeRow(index: number) {
    onChange({
      ...group,
      conditions: rows.filter((_, i) => i !== index),
    });
  }

  function addRow() {
    onChange({
      ...group,
      conditions: [
        ...rows,
        { field: "", operator: "equals", value: "" },
      ],
    });
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <FormSelect
          className="h-8 w-auto min-w-[4.5rem] text-xs"
          value={group.combinator ?? "AND"}
          options={[
            { value: "AND", label: "AND" },
            { value: "OR", label: "OR" },
          ]}
          onChange={(next) =>
            onChange({
              ...group,
              combinator: next as "AND" | "OR",
            })
          }
        />
        <div className="flex items-center gap-1.5">
          <Switch
            size="sm"
            checked={Boolean(group.ignoreCase)}
            onCheckedChange={(checked) =>
              onChange({ ...group, ignoreCase: checked })
            }
          />
          <Label className="text-[11px] text-muted-foreground">
            Ignore case
          </Label>
        </div>
      </div>
      {rows.map((row, i) => (
        <div key={i} className="flex flex-wrap items-center gap-1">
          <div className="min-w-[120px] flex-1">
            <ExpressionField
              placeholder="Field path"
              value={String(row.field ?? "")}
              onChange={(next) => updateRow(i, { field: next })}
            />
          </div>
          <FormSelect
            className="h-8 w-auto min-w-[8rem] text-xs"
            value={String(row.operator ?? "equals")}
            options={COMPARE_OPS}
            onChange={(next) => updateRow(i, { operator: next })}
          />
          <div className="min-w-[120px] flex-1">
            <ExpressionField
              placeholder="Value"
              value={String(row.value ?? "")}
              onChange={(next) => updateRow(i, { value: next })}
            />
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            onClick={() => removeRow(i)}
            aria-label="Remove condition"
          >
            <Trash2 />
          </Button>
        </div>
      ))}
      <Button type="button" variant="outline" size="xs" onClick={addRow}>
        <Plus />
        Add condition
      </Button>
    </div>
  );
}

export type AssignmentRow = {
  name?: string;
  value?: unknown;
  type?: string;
};

export function AssignmentsEditor({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (next: AssignmentRow[] | Record<string, string>) => void;
}) {
  const rows: AssignmentRow[] = Array.isArray(value)
    ? (value as AssignmentRow[])
    : value && typeof value === "object"
      ? Object.entries(value as Record<string, unknown>).map(([name, v]) => ({
          name,
          value: v,
          type: "string",
        }))
      : [];

  function update(index: number, patch: Partial<AssignmentRow>) {
    const next = rows.map((r, i) => (i === index ? { ...r, ...patch } : r));
    onChange(next);
  }

  function remove(index: number) {
    onChange(rows.filter((_, i) => i !== index));
  }

  function add() {
    onChange([...rows, { name: "", value: "", type: "string" }]);
  }

  return (
    <div className="flex flex-col gap-1.5">
      {rows.map((row, i) => (
        <div key={i} className="flex items-center gap-1">
          <Input
            className="h-8 text-xs"
            placeholder="Name"
            value={String(row.name ?? "")}
            onChange={(e) => update(i, { name: e.target.value })}
          />
          <FormSelect
            className="h-8 w-auto min-w-[5.5rem] text-xs"
            value={String(row.type ?? "string")}
            options={[
              { value: "string", label: "String" },
              { value: "number", label: "Number" },
              { value: "boolean", label: "Boolean" },
              { value: "array", label: "Array" },
              { value: "object", label: "Object" },
            ]}
            onChange={(next) => update(i, { type: next })}
          />
          <div className="min-w-0 flex-1">
            <ExpressionField
              placeholder="Value"
              value={String(row.value ?? "")}
              onChange={(next) => update(i, { value: next })}
            />
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            onClick={() => remove(i)}
            aria-label="Remove"
          >
            <Trash2 />
          </Button>
        </div>
      ))}
      <Button type="button" variant="outline" size="xs" onClick={add}>
        <Plus />
        Add field
      </Button>
    </div>
  );
}
