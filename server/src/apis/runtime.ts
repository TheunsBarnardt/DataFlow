import type { FastifyInstance, FastifyReply, FastifyRequest } from "fastify";
import type { ApiEndpointDoc, ApiProjectDoc, WorkflowDoc } from "@dataflow/shared";
import {
  apiProjects,
  apiEndpoints,
  workflows,
  serialize,
  toId,
} from "../db.js";
import { executeWorkflow } from "../engine/executor.js";
import { errorMessage } from "../errorMessage.js";
import { buildOpenApiDocument, swaggerUiHtml } from "./openapi.js";
import { fromNodeHeaders } from "better-auth/node";
import { getAuth } from "../auth/auth.js";
import { apiKeyFromRequest, bearerFromRequest, verifyJwt } from "./jwt.js";

type ApiKeyMeta = {
  projectId?: string;
  scopes?: { method: string; pathPattern: string }[];
  readonly?: boolean;
};

function pathAllowed(
  method: string,
  path: string,
  meta: ApiKeyMeta | undefined,
): boolean {
  if (meta?.readonly && !["GET", "HEAD", "OPTIONS"].includes(method.toUpperCase())) {
    return false;
  }
  const scopes = meta?.scopes;
  if (!scopes || scopes.length === 0) return true;
  return scopes.some((s) => {
    if (s.method.toUpperCase() !== method.toUpperCase() && s.method !== "*") {
      return false;
    }
    const pattern = s.pathPattern.replace(/\*/g, ".*");
    return new RegExp(`^${pattern}$`, "i").test(path);
  });
}

function shortBodySummary(body: unknown): string {
  try {
    const raw = typeof body === "string" ? body : JSON.stringify(body ?? null);
    return raw.length > 240 ? `${raw.slice(0, 240)}…` : raw;
  } catch {
    return "[unserializable]";
  }
}

function extractRespond(runOutput: unknown): {
  statusCode: number;
  body: unknown;
  headers?: Record<string, string>;
} {
  if (
    runOutput &&
    typeof runOutput === "object" &&
    "statusCode" in runOutput &&
    "body" in (runOutput as object)
  ) {
    const r = runOutput as {
      statusCode?: number;
      body?: unknown;
    };
    const body = r.body;
    if (
      body &&
      typeof body === "object" &&
      !Array.isArray(body) &&
      "headers" in body &&
      "body" in body
    ) {
      const wrapped = body as {
        headers?: Record<string, string>;
        body?: unknown;
      };
      return {
        statusCode: Number(r.statusCode ?? 200),
        body: wrapped.body,
        headers: wrapped.headers,
      };
    }
    return {
      statusCode: Number(r.statusCode ?? 200),
      body,
    };
  }
  return { statusCode: 200, body: runOutput ?? null };
}

/** Match OpenAPI-style templates e.g. Opportunity/{id}/Won → Opportunity/guid/Won */
function matchPathTemplate(
  template: string,
  actual: string,
): Record<string, string> | null {
  const tParts = template.split("/").filter(Boolean);
  const aParts = actual.split("/").filter(Boolean);
  if (tParts.length !== aParts.length) return null;
  const params: Record<string, string> = {};
  for (let i = 0; i < tParts.length; i++) {
    const t = tParts[i]!;
    const a = aParts[i]!;
    const m = /^\{([^}]+)\}$/.exec(t);
    if (m) {
      params[m[1]!] = a;
      continue;
    }
    // Allow trailing literal after param token e.g. {referenceType}s
    const mixed = /^\{([^}]+)\}(.+)$/.exec(t);
    if (mixed) {
      const suffix = mixed[2]!;
      if (!a.endsWith(suffix) || a.length <= suffix.length) return null;
      params[mixed[1]!] = a.slice(0, -suffix.length);
      continue;
    }
    if (t !== a) return null;
  }
  return params;
}

async function resolveEndpoint(
  slug: string,
  method: string,
  restPath: string,
): Promise<{
  project: ApiProjectDoc & { _id: string };
  endpoint: ApiEndpointDoc & { _id: string };
  workflow: WorkflowDoc & { _id: string };
  pathParams: Record<string, string>;
} | null> {
  const project = await apiProjects().findOne({ slug, enabled: true });
  if (!project) return null;
  const path = restPath.replace(/^\/+/, "").replace(/\/+$/, "");
  const methodUpper = method.toUpperCase() as ApiEndpointDoc["method"];

  let endpoint = await apiEndpoints().findOne({
    apiProjectId: String(project._id),
    method: methodUpper,
    path,
  });
  let pathParams: Record<string, string> = {};

  if (!endpoint) {
    const candidates = await apiEndpoints()
      .find({
        apiProjectId: String(project._id),
        method: methodUpper,
        path: { $regex: "\\{" },
      })
      .toArray();
    for (const cand of candidates) {
      const matched = matchPathTemplate(cand.path, path);
      if (matched) {
        endpoint = cand;
        pathParams = matched;
        break;
      }
    }
  }

  if (!endpoint) return null;
  const workflow = await workflows().findOne({
    _id: toId(endpoint.workflowId),
  });
  if (!workflow || !workflow.enabled) return null;
  return {
    project: serialize(project) as ApiProjectDoc & { _id: string },
    endpoint: serialize(endpoint) as ApiEndpointDoc & { _id: string },
    workflow: serialize(workflow) as WorkflowDoc & { _id: string },
    pathParams,
  };
}

export async function registerApiProjectRoutes(app: FastifyInstance) {
  // OpenAPI JSON
  app.get<{ Params: { slug: string } }>(
    "/apis/:slug/openapi.json",
    async (req, reply) => {
      const project = await apiProjects().findOne({ slug: req.params.slug });
      if (!project) return reply.code(404).send({ error: "API not found" });
      const endpoints = await apiEndpoints()
        .find({ apiProjectId: String(project._id) })
        .toArray();
      return buildOpenApiDocument(
        serialize(project) as ApiProjectDoc,
        endpoints.map((e) => serialize(e) as ApiEndpointDoc),
      );
    },
  );

  // Swagger UI
  app.get<{ Params: { slug: string } }>(
    "/apis/:slug/swagger",
    async (req, reply) => {
      const project = await apiProjects().findOne({ slug: req.params.slug });
      if (!project) return reply.code(404).send({ error: "API not found" });
      const specUrl = `/apis/${req.params.slug}/openapi.json`;
      reply.type("text/html").send(
        swaggerUiHtml(specUrl, project.name),
      );
    },
  );

  // Catch-all endpoint executor
  app.all<{ Params: { slug: string; "*": string } }>(
    "/apis/:slug/*",
    async (req, reply) => {
      const rest = String((req.params as { "*": string })["*"] ?? "");
      // Reserve docs paths (already registered above, but guard anyway)
      if (rest === "openapi.json" || rest === "swagger") {
        return reply.code(404).send({ error: "Not found" });
      }

      const resolved = await resolveEndpoint(
        req.params.slug,
        req.method,
        rest,
      );
      if (!resolved) {
        return reply.code(404).send({
          message: `No endpoint ${req.method} /${rest} on API ${req.params.slug}`,
        });
      }

      const { project, endpoint, workflow, pathParams } = resolved;

      // Auth — modes match apiEndpoint NDV Authentication select
      if (endpoint.auth === "apiKey") {
        const key = apiKeyFromRequest(
          req.headers as Record<string, unknown>,
        );
        if (!key) {
          return reply.code(401).send({ message: "Unauthorized" });
        }
        let ok = false;
        try {
          const verified = await getAuth().api.verifyApiKey({
            body: { key },
          });
          if (verified.valid && verified.key) {
            const meta = (verified.key.metadata ?? {}) as ApiKeyMeta;
            if (
              meta.projectId &&
              meta.projectId !== String(project._id) &&
              meta.projectId !== project.slug
            ) {
              return reply.code(403).send({ message: "Forbidden" });
            }
            if (!pathAllowed(req.method, rest, meta)) {
              return reply.code(403).send({ message: "Forbidden" });
            }
            ok = true;
          }
        } catch {
          /* fall through to project secret */
        }
        if (!ok) {
          const expected = project.auth?.jwtSecret ?? "";
          if (!expected || key !== expected) {
            return reply.code(401).send({ message: "Unauthorized" });
          }
        }
      } else if (endpoint.auth === "jwt" || endpoint.auth === "headerBearer") {
        const token = bearerFromRequest(
          req.headers as Record<string, unknown>,
        );
        if (!token) {
          return reply.code(401).send({ message: "Unauthorized" });
        }
        if (endpoint.auth === "headerBearer") {
          const expected = project.auth?.jwtSecret ?? "";
          if (!expected || token !== expected) {
            return reply.code(401).send({ message: "Unauthorized" });
          }
        } else {
          // Prefer Better Auth session; fall back to project HS256 JWT
          let sessionOk = false;
          try {
            const session = await getAuth().api.getSession({
              headers: fromNodeHeaders(req.headers),
            });
            if (session?.user) {
              sessionOk = true;
              (req as FastifyRequest & { apiJwt?: unknown }).apiJwt = {
                sub: session.user.id,
                email: session.user.email,
              };
            }
          } catch {
            /* ignore */
          }
          if (!sessionOk) {
            const secret = project.auth?.jwtSecret ?? "dataflow-dev-secret";
            const verified = verifyJwt(token, secret);
            if (!verified.ok) {
              return reply.code(401).send({ message: verified.error });
            }
            (req as FastifyRequest & { apiJwt?: unknown }).apiJwt =
              verified.payload;
          }
        }
      }

      try {
        const run = await executeWorkflow({
          workflowId: workflow._id,
          trigger: "api",
          input: {
            kind: "api",
            apiProjectId: project._id,
            apiSlug: project.slug,
            method: req.method,
            path: rest,
            headers: req.headers,
            query: { ...(req.query as object), ...pathParams },
            params: pathParams,
            body: req.body,
            jwt: (req as FastifyRequest & { apiJwt?: unknown }).apiJwt,
          },
          actor: "api",
        });

        if (run.status === "failed") {
          const message = run.error ?? "An unexpected error occurred.";
          console.error(
            JSON.stringify({
              msg: "API endpoint run failed",
              slug: project.slug,
              method: req.method,
              path: rest,
              runId: String(run._id),
              error: message,
              statusCode: 500,
            }),
          );
          return reply.code(500).send({ message });
        }

        const respond = extractRespond(run.output);
        if (respond.statusCode >= 400) {
          console.error(
            JSON.stringify({
              msg: "API endpoint non-success response",
              slug: project.slug,
              method: req.method,
              path: rest,
              runId: String(run._id),
              statusCode: respond.statusCode,
              bodySummary: shortBodySummary(respond.body),
            }),
          );
        }
        if (respond.headers) {
          for (const [k, v] of Object.entries(respond.headers)) {
            reply.header(k, v);
          }
        }
        return reply.code(respond.statusCode).send(respond.body);
      } catch (err) {
        const message = errorMessage(err) || "An unexpected error occurred.";
        console.error(
          JSON.stringify({
            msg: "API endpoint handler error",
            slug: req.params.slug,
            method: req.method,
            path: rest,
            error: message,
            statusCode: 500,
          }),
        );
        return reply.code(500).send({ message });
      }
    },
  );
}

/** Used by management routes to re-check slug uniqueness etc. */
export async function findApiProjectBySlug(slug: string) {
  return apiProjects().findOne({ slug });
}
