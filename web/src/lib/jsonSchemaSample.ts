/** Minimal JSON Schema helpers for API endpoint mapping UX. */

export type JsonSchema = Record<string, unknown>;

const SCHEMA_TYPES = [
  "string",
  "integer",
  "number",
  "boolean",
  "array",
  "object",
] as const;

export type SchemaPropType = (typeof SCHEMA_TYPES)[number];

export const SCHEMA_PROP_TYPE_OPTIONS: {
  value: SchemaPropType;
  label: string;
}[] = [
  { value: "string", label: "String" },
  { value: "integer", label: "Integer" },
  { value: "number", label: "Number" },
  { value: "boolean", label: "Boolean" },
  { value: "array", label: "Array" },
  { value: "object", label: "Object" },
];

export type SchemaPropertyRow = {
  name: string;
  type: SchemaPropType;
  /** Example / sample value shown in Input mapping (Postman-style). */
  example: string;
  required: boolean;
  description?: string;
};

export function isPlainSchemaObject(
  value: unknown,
): value is JsonSchema {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

/** True when schema is a plain object with properties (not $ref / array). */
export function isEditableObjectSchema(schema: unknown): boolean {
  if (!isPlainSchemaObject(schema)) return false;
  if ("$ref" in schema) return false;
  if (schema.type === "array") return false;
  // Empty / missing type is fine — treat as object properties schema
  if (schema.type != null && schema.type !== "object") return false;
  return true;
}

function exampleFromProp(prop: JsonSchema): string {
  if (prop.default !== undefined && prop.default !== null) {
    return typeof prop.default === "string"
      ? prop.default
      : JSON.stringify(prop.default);
  }
  if (prop.example !== undefined && prop.example !== null) {
    return typeof prop.example === "string"
      ? prop.example
      : JSON.stringify(prop.example);
  }
  if (Array.isArray(prop.examples) && prop.examples.length > 0) {
    const first = prop.examples[0];
    return typeof first === "string" ? first : JSON.stringify(first);
  }
  return "";
}

function coerceExample(type: SchemaPropType, raw: string): unknown {
  const trimmed = raw.trim();
  if (!trimmed) {
    if (type === "integer" || type === "number") return 0;
    if (type === "boolean") return false;
    if (type === "array") return [];
    if (type === "object") return {};
    return "";
  }
  if (type === "integer") {
    const n = Number.parseInt(trimmed, 10);
    return Number.isFinite(n) ? n : 0;
  }
  if (type === "number") {
    const n = Number(trimmed);
    return Number.isFinite(n) ? n : 0;
  }
  if (type === "boolean") {
    return /^(true|1|yes)$/i.test(trimmed);
  }
  if (type === "array" || type === "object") {
    try {
      return JSON.parse(trimmed) as unknown;
    } catch {
      return type === "array" ? [trimmed] : { value: trimmed };
    }
  }
  return trimmed;
}

export function schemaToPropertyRows(schema: unknown): SchemaPropertyRow[] {
  if (!isEditableObjectSchema(schema)) return [];
  const props =
    (schema.properties as Record<string, JsonSchema> | undefined) ?? {};
  const required = new Set(
    Array.isArray(schema.required)
      ? schema.required.map(String)
      : [],
  );
  return Object.entries(props).map(([name, prop]) => {
    const rawType = String(prop?.type ?? "string");
    const type = (SCHEMA_TYPES as readonly string[]).includes(rawType)
      ? (rawType as SchemaPropType)
      : "string";
    return {
      name,
      type,
      example: exampleFromProp(prop),
      required: required.has(name),
      description:
        typeof prop.description === "string" ? prop.description : undefined,
    };
  });
}

export function propertyRowsToSchema(
  rows: SchemaPropertyRow[],
): JsonSchema {
  const properties: Record<string, JsonSchema> = {};
  const required: string[] = [];
  for (const row of rows) {
    const name = row.name.trim();
    if (!name) continue;
    const prop: JsonSchema = { type: row.type };
    if (row.example.trim()) {
      prop.default = coerceExample(row.type, row.example);
      prop.example = prop.default;
    }
    if (row.description?.trim()) {
      prop.description = row.description.trim();
    }
    properties[name] = prop;
    if (row.required) required.push(name);
  }
  return {
    type: "object",
    properties,
    required,
  };
}

/** Placeholder value for a property schema — used for mapping previews. */
export function sampleValueFromSchema(schema: unknown): unknown {
  if (!isPlainSchemaObject(schema)) return null;
  if ("$ref" in schema) return {};

  // Prefer Postman-style example / default when present
  if (schema.default !== undefined) return schema.default;
  if (schema.example !== undefined) return schema.example;
  if (Array.isArray(schema.examples) && schema.examples.length > 0) {
    return schema.examples[0];
  }

  const type = schema.type;

  if (type === "array") {
    const items = schema.items;
    if (isPlainSchemaObject(items)) {
      const one = sampleValueFromSchema(items);
      return one === null || one === undefined ? [] : [one];
    }
    return [];
  }

  if (type === "object" || schema.properties) {
    const props =
      (schema.properties as Record<string, JsonSchema> | undefined) ?? {};
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(props)) {
      out[k] = sampleValueFromSchema(v);
    }
    return out;
  }

  if (type === "integer" || type === "number") return 0;
  if (type === "boolean") return false;
  if (type === "string") {
    const format = String(schema.format ?? "");
    if (format === "uuid") return "00000000-0000-0000-0000-000000000000";
    if (format === "date-time") return "2026-01-01T00:00:00.000Z";
    if (format === "date") return "2026-01-01";
    return "";
  }

  return null;
}

/**
 * n8n-style sample item for an apiEndpoint trigger, derived from its
 * request schemas so downstream nodes can map `$json.query.*` / `$json.body.*`
 * before a real request runs.
 */
export function sampleFromApiEndpointParameters(
  parameters: Record<string, unknown> | undefined | null,
): unknown[] | undefined {
  if (!parameters) return undefined;
  const querySchema = parameters.requestQuerySchema;
  const bodySchema = parameters.requestBodySchema;
  const queryProps = isPlainSchemaObject(querySchema)
    ? (querySchema.properties as Record<string, unknown> | undefined)
    : undefined;
  const bodyProps = isPlainSchemaObject(bodySchema)
    ? (bodySchema.properties as Record<string, unknown> | undefined)
    : undefined;
  const hasQuery =
    (queryProps != null && Object.keys(queryProps).length > 0) ||
    (isPlainSchemaObject(querySchema) && Boolean(querySchema.$ref));
  const hasBody =
    (bodyProps != null && Object.keys(bodyProps).length > 0) ||
    (isPlainSchemaObject(bodySchema) &&
      (Boolean(bodySchema.$ref) || bodySchema.type === "array"));

  if (!hasQuery && !hasBody) return undefined;

  const query = hasQuery
    ? (sampleValueFromSchema(querySchema) as Record<string, unknown>) ?? {}
    : {};
  const body = hasBody ? sampleValueFromSchema(bodySchema) : null;

  return [
    {
      method: String(parameters.method ?? "GET"),
      path: String(parameters.path ?? ""),
      headers: {},
      query,
      body,
    },
  ];
}
