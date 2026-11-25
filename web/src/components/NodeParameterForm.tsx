import { AlignLeft, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { FormSelect } from "@/components/ui/form-select";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  VariableAwareInput,
  VariableAwareTextarea,
} from "@/components/VariableAwareField";
import {
  CodeEditor,
  type CodeEditorLanguage,
} from "@/components/CodeEditor";
import { formatCode, type CodeLanguage } from "@/lib/formatCode";
import {
  fieldVisible,
  type FieldDef,
  type NodeSchema,
} from "@/lib/nodeSchemas";
import { cn } from "@/lib/utils";
import {
  AssignmentsEditor,
  ConditionBuilder,
} from "@/components/ConditionBuilder";
import { ExpressionField } from "@/components/ExpressionField";
import { SwitchRulesEditor } from "@/components/SwitchRulesEditor";
import {
  CredentialsPicker,
  isCredentialField,
} from "@/components/CredentialsPicker";
import { JsonSchemaPropertiesEditor } from "@/components/JsonSchemaPropertiesEditor";

function FormatButton({
  value,
  language,
  onChange,
}: {
  value: string;
  language: CodeLanguage;
  onChange: (next: string) => void;
}) {
  if (language !== "sql" && language !== "json") return null;
  return (
    <Button
      type="button"
      variant="ghost"
      size="xs"
      className="h-6 gap-1 px-1.5 text-[10px] text-muted-foreground"
      title="Format"
      onClick={() => {
        const result = formatCode(value, language);
        if (!result.ok) {
          toast.error(result.error);
          return;
        }
        onChange(result.text);
      }}
    >
      <AlignLeft className="size-3" />
      Format
    </Button>
  );
}

function KeyValueEditor({
  value,
  onChange,
}: {
  value: Record<string, string>;
  onChange: (next: Record<string, string>) => void;
}) {
  const entries = Object.entries(value);

  function update(index: number, key: string, val: string) {
    const next = [...entries];
    next[index] = [key, val];
    onChange(Object.fromEntries(next.filter(([k]) => k.trim() !== "")));
  }

  function remove(index: number) {
    const next = entries.filter((_, i) => i !== index);
    onChange(Object.fromEntries(next));
  }

  function add() {
    onChange({ ...value, "": "" });
  }

  return (
    <div className="flex flex-col gap-1.5">
      {entries.map(([k, v], i) => (
        <div key={i} className="flex items-center gap-1">
          <Input
            className="h-8 text-xs"
            placeholder="Key"
            value={k}
            onChange={(e) => update(i, e.target.value, v)}
          />
          <VariableAwareInput
            className="h-8 text-xs"
            placeholder="Value ($ for vars)"
            value={v}
            onChange={(next) => update(i, k, next)}
          />
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            onClick={() => remove(i)}
            aria-label="Remove row"
          >
            <Trash2 />
          </Button>
        </div>
      ))}
      <Button type="button" variant="outline" size="xs" onClick={add}>
        <Plus />
        Add pair
      </Button>
    </div>
  );
}

function FieldControl({
  field,
  value,
  onChange,
}: {
  field: FieldDef;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  const id = `param-${field.key}`;

  if (isCredentialField(field.key)) {
    return (
      <CredentialsPicker
        id={id}
        value={String(value ?? "")}
        onChange={onChange}
        credentialType={field.credentialType}
      />
    );
  }

  if (field.kind === "boolean") {
    return (
      <div className="flex items-center gap-2">
        <Switch
          id={id}
          checked={Boolean(value)}
          onCheckedChange={(checked) => onChange(checked)}
        />
        <Label htmlFor={id}>{field.label}</Label>
      </div>
    );
  }

  if (field.kind === "select") {
    return (
      <FormSelect
        id={id}
        value={String(value ?? field.default ?? "")}
        options={field.options ?? []}
        onChange={onChange}
      />
    );
  }

  if (field.kind === "number") {
    return (
      <Input
        id={id}
        type="number"
        value={value === undefined || value === null ? "" : String(value)}
        placeholder={field.placeholder}
        onChange={(e) => {
          const n = e.target.value === "" ? undefined : Number(e.target.value);
          onChange(n);
        }}
      />
    );
  }

  if (field.kind === "text") {
    const text = String(value ?? "");
    return (
      <VariableAwareTextarea
        id={id}
        className="min-h-[88px] text-xs"
        value={text}
        placeholder={field.placeholder ?? "Type $ to insert a variable"}
        spellCheck={false}
        onChange={onChange}
      />
    );
  }

  if (field.kind === "code") {
    const text = String(value ?? "");
    const language = (field.language ?? "text") as CodeLanguage;
    const editorLang: CodeEditorLanguage =
      language === "sql"
        ? "sql"
        : language === "json"
          ? "json"
          : language === "js"
            ? "javascript"
            : "plaintext";
    const height = language === "sql" ? 160 : 120;
    return (
      <div className="flex flex-col gap-1">
        <div className="flex justify-end">
          <FormatButton
            value={text}
            language={language}
            onChange={(next) => onChange(next)}
          />
        </div>
        <CodeEditor
          id={id}
          value={text}
          language={editorLang}
          height={height}
          placeholder={field.placeholder ?? "Type $ to insert a variable"}
          onChange={onChange}
        />
      </div>
    );
  }

  if (field.kind === "json") {
    const text =
      typeof value === "string"
        ? value
        : value === undefined || value === null
          ? ""
          : JSON.stringify(value, null, 2);
    return (
      <div className="flex flex-col gap-1">
        <div className="flex justify-end">
          <FormatButton
            value={text}
            language="json"
            onChange={(next) => {
              try {
                onChange(JSON.parse(next) as unknown);
              } catch {
                onChange(next);
              }
            }}
          />
        </div>
        <CodeEditor
          id={id}
          value={text}
          language="json"
          height={120}
          placeholder={field.placeholder ?? "{ }"}
          onChange={(next) => {
            const raw = next.trim();
            if (!raw) {
              onChange(undefined);
              return;
            }
            try {
              onChange(JSON.parse(raw) as unknown);
            } catch {
              // Keep raw string until valid; parent validates on apply
              onChange(next);
            }
          }}
        />
      </div>
    );
  }

  if (field.kind === "jsonSchema") {
    return <JsonSchemaPropertiesEditor value={value} onChange={onChange} />;
  }

  if (field.kind === "keyValue") {
    const obj =
      value && typeof value === "object" && !Array.isArray(value)
        ? Object.fromEntries(
            Object.entries(value as Record<string, unknown>).map(([k, v]) => [
              k,
              String(v ?? ""),
            ]),
          )
        : {};
    return <KeyValueEditor value={obj} onChange={onChange} />;
  }

  if (field.kind === "stringList") {
    const list = Array.isArray(value) ? (value as unknown[]).map(String) : [];
    return (
      <Textarea
        id={id}
        className="min-h-[72px] font-mono text-xs"
        value={list.join("\n")}
        placeholder={field.placeholder ?? "One per line"}
        spellCheck={false}
        onChange={(e) =>
          onChange(
            e.target.value
              .split(/\r?\n/)
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
      />
    );
  }

  if (field.kind === "conditions") {
    return <ConditionBuilder value={value} onChange={onChange} />;
  }

  if (field.kind === "assignments") {
    return <AssignmentsEditor value={value} onChange={onChange} />;
  }

  if (field.kind === "switchRules") {
    return <SwitchRulesEditor value={value} onChange={onChange} />;
  }

  return (
    <ExpressionField
      id={id}
      value={String(value ?? "")}
      placeholder={field.placeholder ?? "Type $ for expressions"}
      onChange={onChange}
    />
  );
}

export function NodeParameterForm({
  schema,
  values,
  onChange,
}: {
  schema: NodeSchema;
  values: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
}) {
  const visible = schema.fields.filter((f) => fieldVisible(f, values));

  function setField(key: string, value: unknown) {
    const next = { ...values };
    if (value === undefined) delete next[key];
    else next[key] = value;
    onChange(next);
  }

  if (!visible.length) {
    return (
      <p className="rounded-md border bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
        This node has no configurable parameters.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {visible.map((field) => (
        <div key={field.key} className="flex flex-col gap-1.5">
          {field.kind !== "boolean" ? (
            <Label
              htmlFor={`param-${field.key}`}
              className="text-[11px] text-muted-foreground"
            >
              {field.label}
            </Label>
          ) : null}
          {field.description ? (
            <p className="text-[11px] text-muted-foreground/80">
              {field.description}
            </p>
          ) : null}
          <FieldControl
            field={field}
            value={values[field.key]}
            onChange={(v) => setField(field.key, v)}
          />
        </div>
      ))}
    </div>
  );
}

/** Validate JSON-ish fields that may still hold raw invalid strings. */
export function sanitizeParameters(
  schema: NodeSchema,
  values: Record<string, unknown>,
): { ok: true; parameters: Record<string, unknown> } | { ok: false; error: string } {
  const out: Record<string, unknown> = { ...values };
  for (const field of schema.fields) {
    if (!fieldVisible(field, values)) {
      delete out[field.key];
      continue;
    }
    if (field.kind === "json" || field.kind === "jsonSchema") {
      const v = out[field.key];
      if (typeof v === "string") {
        const raw = v.trim();
        if (!raw) {
          delete out[field.key];
          continue;
        }
        try {
          out[field.key] = JSON.parse(raw) as unknown;
        } catch {
          return { ok: false, error: `Invalid JSON in ${field.label}` };
        }
      }
    }
    if (field.kind === "number" && out[field.key] !== undefined) {
      const n = Number(out[field.key]);
      if (!Number.isFinite(n)) {
        return { ok: false, error: `${field.label} must be a number` };
      }
      out[field.key] = n;
    }
  }
  return { ok: true, parameters: out };
}
