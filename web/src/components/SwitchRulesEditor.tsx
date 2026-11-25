import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  ConditionBuilder,
  type ConditionGroupValue,
} from "@/components/ConditionBuilder";

export type SwitchRule = {
  outputKey?: string;
  conditions?: ConditionGroupValue | unknown[];
  combinator?: string;
};

function normalize(value: unknown): SwitchRule[] {
  if (!Array.isArray(value)) return [];
  return value as SwitchRule[];
}

export function SwitchRulesEditor({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (next: SwitchRule[]) => void;
}) {
  const rules = normalize(value);

  function update(index: number, patch: Partial<SwitchRule>) {
    onChange(rules.map((r, i) => (i === index ? { ...r, ...patch } : r)));
  }

  function remove(index: number) {
    onChange(rules.filter((_, i) => i !== index));
  }

  function add() {
    onChange([
      ...rules,
      {
        outputKey: `output${rules.length}`,
        conditions: {
          combinator: "AND",
          conditions: [{ field: "", operator: "equals", value: "" }],
        },
      },
    ]);
  }

  return (
    <div className="flex flex-col gap-3">
      {rules.map((rule, i) => (
        <div key={i} className="rounded-md border bg-muted/20 p-2">
          <div className="mb-2 flex items-center gap-2">
            <Input
              className="h-8 text-xs"
              placeholder="Output key"
              value={String(rule.outputKey ?? "")}
              onChange={(e) => update(i, { outputKey: e.target.value })}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon-xs"
              onClick={() => remove(i)}
              aria-label="Remove rule"
            >
              <Trash2 />
            </Button>
          </div>
          <ConditionBuilder
            value={
              rule.conditions ?? {
                combinator: "AND",
                conditions: [],
              }
            }
            onChange={(next) => update(i, { conditions: next })}
          />
        </div>
      ))}
      <Button type="button" variant="outline" size="xs" onClick={add}>
        <Plus />
        Add rule
      </Button>
    </div>
  );
}
