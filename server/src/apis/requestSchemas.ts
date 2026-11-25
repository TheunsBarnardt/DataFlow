/**
 * Request query/body schemas sourced from original ASP.NET projects
 * (scripts/extract-request-schemas.py → requestSchemas.generated.ts).
 */
import type { JsonSchema } from "@dataflow/shared";
import { EXTRACTED_REQUEST_SCHEMAS } from "./requestSchemas.generated.js";

export type ApiMirrorProject = "wealth" | "wealthOnline" | "dynamics";

export type ExtractedEndpointSchemas = {
  controller: string;
  action: string;
  method: string;
  query: Record<string, JsonSchema>;
  body: Record<string, JsonSchema>;
};

export type ExtractedRequestSchemaMap = Record<
  ApiMirrorProject,
  Record<string, ExtractedEndpointSchemas>
>;

export function operationKey(controller: string, action: string): string {
  return `${controller}_${action}`;
}

function propsToSchema(
  props: Record<string, JsonSchema> | undefined,
): JsonSchema | undefined {
  if (!props || Object.keys(props).length === 0) return undefined;
  return {
    type: "object",
    properties: props,
    required: [],
  };
}

/** Flat Postman-style schemas from the extracted C# surface. */
export function resolveExtractedRequestSchemas(
  project: ApiMirrorProject,
  controller: string,
  action: string,
): {
  requestQuerySchema?: JsonSchema;
  requestBodySchema?: JsonSchema;
} {
  const entry =
    EXTRACTED_REQUEST_SCHEMAS[project]?.[operationKey(controller, action)];
  if (!entry) return {};
  return {
    requestQuerySchema: propsToSchema(entry.query),
    requestBodySchema: propsToSchema(entry.body),
  };
}

/** Merge extracted schemas onto a catalog endpoint def (extract wins when present). */
export function withExtractedRequestSchemas<
  T extends {
    controller: string;
    action: string;
    requestQuerySchema?: JsonSchema;
    requestBodySchema?: JsonSchema;
  },
>(project: ApiMirrorProject, def: T): T {
  const extracted = resolveExtractedRequestSchemas(
    project,
    def.controller,
    def.action,
  );
  return {
    ...def,
    requestQuerySchema:
      extracted.requestQuerySchema ?? def.requestQuerySchema,
    requestBodySchema: extracted.requestBodySchema ?? def.requestBodySchema,
  };
}
