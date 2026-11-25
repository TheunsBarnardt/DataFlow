import type { FastifyInstance } from "fastify";
import { ObjectId } from "mongodb";
import { nanoid } from "nanoid";
import type {
  ApiProjectDoc,
  WorkflowDoc,
  WorkflowNode,
  WorkflowEdge,
} from "@dataflow/shared";
import {
  apiProjects,
  apiEndpoints,
  workflows,
  toId,
  serialize,
  type Stored,
} from "../db.js";
import { actorFromRequest } from "../auth/actor.js";
import {
  syncApiEndpointsFromWorkflow,
  deleteApiEndpointsForProject,
  deleteApiEndpointsForWorkflow,
} from "../apis/sync.js";
import { buildOpenApiDocument } from "../apis/openapi.js";

function slugify(raw: string): string {
  return raw
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}

export async function registerApiProjectMgmtRoutes(app: FastifyInstance) {
  app.get("/api/api-projects", async () => {
    const list = await apiProjects().find({}).sort({ updatedAt: -1 }).toArray();
    return { projects: list.map(serialize) };
  });

  app.get<{ Params: { id: string } }>(
    "/api/api-projects/:id",
    async (req, reply) => {
      const project = await apiProjects().findOne({ _id: toId(req.params.id) });
      if (!project) return reply.code(404).send({ error: "Not found" });
      const endpoints = await apiEndpoints()
        .find({ apiProjectId: String(project._id) })
        .toArray();
      const endpointWorkflows = await workflows()
        .find({
          apiProjectId: String(project._id),
          kind: "apiEndpoint",
        })
        .sort({ name: 1 })
        .toArray();
      return {
        project: serialize(project),
        endpoints: endpoints.map(serialize),
        workflows: endpointWorkflows.map(serialize),
      };
    },
  );

  app.post<{ Body: Partial<ApiProjectDoc> }>(
    "/api/api-projects",
    async (req, reply) => {
      const name = String(req.body.name ?? "").trim() || "New API";
      let slug = slugify(String(req.body.slug ?? name));
      if (!slug) slug = `api-${nanoid(8)}`;
      const existing = await apiProjects().findOne({ slug });
      if (existing) {
        return reply.code(409).send({ error: `Slug '${slug}' already exists` });
      }
      const now = new Date().toISOString();
      const actor = actorFromRequest(req);
      const doc: Stored<ApiProjectDoc> = {
        _id: new ObjectId(),
        name,
        slug,
        description: req.body.description ?? "",
        version: req.body.version ?? "v1",
        enabled: req.body.enabled ?? true,
        auth: req.body.auth ?? { mode: "jwt", jwtSecret: nanoid(32) },
        schemas: req.body.schemas ?? {},
        createdBy: actor,
        updatedBy: actor,
        createdAt: now,
        updatedAt: now,
      };
      await apiProjects().insertOne(doc);
      return { project: serialize(doc) };
    },
  );

  app.put<{ Params: { id: string }; Body: Partial<ApiProjectDoc> }>(
    "/api/api-projects/:id",
    async (req, reply) => {
      const existing = await apiProjects().findOne({ _id: toId(req.params.id) });
      if (!existing) return reply.code(404).send({ error: "Not found" });
      const actor = actorFromRequest(req);
      let slug = existing.slug;
      if (req.body.slug !== undefined) {
        slug = slugify(String(req.body.slug));
        if (!slug) {
          return reply.code(400).send({ error: "Invalid slug" });
        }
        const clash = await apiProjects().findOne({
          slug,
          _id: { $ne: existing._id },
        });
        if (clash) {
          return reply.code(409).send({ error: `Slug '${slug}' already exists` });
        }
      }
      const $set = {
        name: req.body.name ?? existing.name,
        slug,
        description:
          req.body.description !== undefined
            ? req.body.description
            : existing.description,
        version: req.body.version ?? existing.version,
        enabled:
          req.body.enabled !== undefined ? req.body.enabled : existing.enabled,
        auth: req.body.auth ?? existing.auth,
        schemas:
          req.body.schemas !== undefined ? req.body.schemas : existing.schemas,
        updatedBy: actor,
        updatedAt: new Date().toISOString(),
      };
      await apiProjects().updateOne({ _id: existing._id }, { $set });
      return { project: serialize({ ...existing, ...$set }) };
    },
  );

  app.delete<{ Params: { id: string } }>(
    "/api/api-projects/:id",
    async (req, reply) => {
      const existing = await apiProjects().findOne({ _id: toId(req.params.id) });
      if (!existing) return reply.code(404).send({ error: "Not found" });
      const projectId = String(existing._id);
      await workflows().deleteMany({
        apiProjectId: projectId,
        kind: "apiEndpoint",
      });
      await deleteApiEndpointsForProject(projectId);
      await apiProjects().deleteOne({ _id: existing._id });
      return { ok: true };
    },
  );

  /** Create a blank endpoint workflow under an API project. */
  app.post<{
    Params: { id: string };
    Body: {
      name?: string;
      path?: string;
      method?: string;
      tag?: string;
      auth?: string;
      summary?: string;
    };
  }>("/api/api-projects/:id/endpoints", async (req, reply) => {
    const project = await apiProjects().findOne({ _id: toId(req.params.id) });
    if (!project) return reply.code(404).send({ error: "Not found" });
    const actor = actorFromRequest(req);
    const now = new Date().toISOString();
    const method = String(req.body.method ?? "GET").toUpperCase();
    const path =
      String(req.body.path ?? "").trim().replace(/^\/+/, "") ||
      `endpoint-${nanoid(6)}`;
    const name =
      String(req.body.name ?? "").trim() || `${method} ${path}`;
    const triggerId = `n_${nanoid(8)}`;
    const respondId = `n_${nanoid(8)}`;
    const nodes: WorkflowNode[] = [
      {
        id: triggerId,
        type: "apiEndpoint",
        name: "API Endpoint",
        position: { x: 80, y: 120 },
        parameters: {
          path,
          method,
          auth: req.body.auth ?? (project.auth?.mode === "jwt" ? "jwt" : "none"),
          tag: req.body.tag ?? path.split("/")[0] ?? "default",
          summary: req.body.summary ?? name,
        },
      },
      {
        id: respondId,
        type: "apiRespond",
        name: "Respond to API",
        position: { x: 360, y: 120 },
        parameters: {
          statusCode: 200,
          responseData: "firstEntryJson",
          responseSchema: { type: "object", properties: {}, required: [] },
        },
      },
    ];
    const edges: WorkflowEdge[] = [
      {
        id: `e_${nanoid(8)}`,
        source: triggerId,
        target: respondId,
      },
    ];
    const doc: Stored<WorkflowDoc> = {
      _id: new ObjectId(),
      name,
      description: "",
      enabled: true,
      version: 1,
      nodes,
      edges,
      tags: [String(req.body.tag ?? path.split("/")[0] ?? "api")],
      kind: "apiEndpoint",
      apiProjectId: String(project._id),
      createdBy: actor,
      updatedBy: actor,
      createdAt: now,
      updatedAt: now,
    };
    await syncApiEndpointsFromWorkflow({
      _id: String(doc._id),
      nodes: doc.nodes,
      enabled: doc.enabled,
      kind: doc.kind,
      apiProjectId: doc.apiProjectId,
    });
    await workflows().insertOne(doc);
    return { workflow: serialize(doc) };
  });

  app.get<{ Params: { id: string } }>(
    "/api/api-projects/:id/openapi.json",
    async (req, reply) => {
      const project = await apiProjects().findOne({ _id: toId(req.params.id) });
      if (!project) return reply.code(404).send({ error: "Not found" });
      const endpoints = await apiEndpoints()
        .find({ apiProjectId: String(project._id) })
        .toArray();
      return buildOpenApiDocument(
        serialize(project) as ApiProjectDoc,
        endpoints.map((e) => serialize(e)),
      );
    },
  );
}

export { deleteApiEndpointsForWorkflow };
