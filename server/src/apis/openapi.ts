import type {
  ApiProjectDoc,
  ApiEndpointDoc,
  JsonSchema,
} from "@dataflow/shared";
import { config } from "../config.js";

function paramsFromSchema(
  schema: JsonSchema | undefined,
  location: "query" | "header",
): Record<string, unknown>[] {
  if (!schema || typeof schema !== "object") return [];
  const props = (schema.properties as Record<string, JsonSchema>) ?? {};
  const required = new Set(
    Array.isArray(schema.required)
      ? schema.required.map(String)
      : [],
  );
  return Object.entries(props).map(([name, prop]) => ({
    name,
    in: location,
    required: required.has(name),
    schema: prop,
    description: String(prop.description ?? ""),
  }));
}

/** Build OpenAPI 3.0 document for an API project + its endpoints. */
export function buildOpenApiDocument(
  project: ApiProjectDoc,
  endpoints: ApiEndpointDoc[],
): Record<string, unknown> {
  const paths: Record<string, Record<string, unknown>> = {};
  const useJwt = project.auth?.mode === "jwt";
  const useApiKey = endpoints.some((e) => e.auth === "apiKey");
  const schemeName = project.auth?.schemeName ?? "Bearer";
  const apiKeyScheme = "ApiKey";

  for (const ep of endpoints) {
    const pathKey = `/${ep.path.replace(/^\/+/, "")}`;
    const method = ep.method.toLowerCase();
    const op: Record<string, unknown> = {
      operationId: ep.operationId ?? `${ep.method}_${ep.path}`,
      summary: ep.summary ?? ep.path,
      tags: [ep.tag ?? "default"],
      parameters: paramsFromSchema(ep.requestQuerySchema, "query"),
      responses: {
        [String(ep.responseStatus ?? 200)]: {
          description: "Success",
          content: ep.responseSchema
            ? {
                "application/json": {
                  schema: ep.responseSchema,
                },
              }
            : undefined,
        },
        "401": { description: "Unauthorized" },
        "500": {
          description: "Error",
          content: {
            "application/json": {
              schema: {
                type: "object",
                properties: { message: { type: "string" } },
              },
            },
          },
        },
      },
    };

    if (ep.requestBodySchema && ["post", "put", "patch"].includes(method)) {
      op.requestBody = {
        required: true,
        content: {
          "application/json": {
            schema: ep.requestBodySchema,
          },
        },
      };
    }

    if (ep.auth === "apiKey") {
      op.security = [{ [apiKeyScheme]: [] }];
    } else if (ep.auth === "jwt" || ep.auth === "headerBearer") {
      op.security = [{ [schemeName]: [] }];
    } else {
      op.security = [];
    }

    if (!paths[pathKey]) paths[pathKey] = {};
    paths[pathKey][method] = op;
  }

  const rawSchemas = { ...(project.schemas ?? {}) };
  delete (rawSchemas as Record<string, unknown>).__seedVersion;
  const components: Record<string, unknown> = {
    schemas: rawSchemas,
  };
  if (useJwt || useApiKey) {
    components.securitySchemes = {
      ...(useJwt
        ? {
            [schemeName]: {
              type: "http",
              scheme: "bearer",
              bearerFormat: "JWT",
            },
          }
        : {}),
      ...(useApiKey
        ? {
            [apiKeyScheme]: {
              type: "apiKey",
              in: "header",
              name: "X-Api-Key",
              description: "API key required (X-Api-Key header).",
            },
          }
        : {}),
    };
  }

  return {
    openapi: "3.0.3",
    info: {
      title: project.name,
      description: project.description ?? "",
      version: project.version || "v1",
    },
    servers: [
      {
        url: `${config.publicBaseUrl}/apis/${project.slug}`,
        description: "DataFlow API project",
      },
    ],
    paths,
    components,
    security: useJwt ? [{ [schemeName]: [] }] : [],
  };
}

export function swaggerUiHtml(specUrl: string, title: string): string {
  const safeTitle = title.replace(/</g, "&lt;");
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>${safeTitle} — Swagger</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui.css" />
  <style>body{margin:0} .topbar{display:none}</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({
      url: ${JSON.stringify(specUrl)},
      dom_id: '#swagger-ui',
      deepLinking: true,
      presets: [SwaggerUIBundle.presets.apis],
      layout: "BaseLayout"
    });
  </script>
</body>
</html>`;
}
