import { ObjectId } from "mongodb";
import type { ApiProjectDoc } from "@dataflow/shared";
import { apiProjects, workflows, type Stored } from "../../db.js";
import { config } from "../../config.js";
import { syncApiEndpointsFromWorkflow } from "../sync.js";
import { reapplyEndpointParameters } from "../reapplyParams.js";
import { DYNAMICS_ENDPOINTS, dynamicsPath } from "./catalog.js";
import { dynamicsSchemas } from "./schemas.js";
import { buildDynamicsFlow } from "./flows.js";

const DYNAMICS_SLUG = "dynamics";
/** Bump to force reseed after catalog/flow changes. */
const DYNAMICS_SEED_VERSION = 1;

export async function ensureDynamicsApiMirror(): Promise<void> {
  const existing = await apiProjects().findOne({ slug: DYNAMICS_SLUG });
  if (existing) {
    const wfs = await workflows()
      .find({ apiProjectId: String(existing._id), kind: "apiEndpoint" })
      .toArray();
    const seedMeta = (
      existing.schemas as Record<string, { const?: unknown }> | undefined
    )?.__seedVersion;
    const seededVersion = Number(seedMeta?.const ?? seedMeta ?? 0);
    const needsReseed =
      seededVersion !== DYNAMICS_SEED_VERSION ||
      wfs.length !== DYNAMICS_ENDPOINTS.length;
    if (!needsReseed) {
      const result = await reapplyEndpointParameters({
        apiProjectId: String(existing._id),
        project: "dynamics",
        endpoints: DYNAMICS_ENDPOINTS,
        pathOf: (d) => dynamicsPath(d as (typeof DYNAMICS_ENDPOINTS)[number]),
      });
      console.log(
        `Reapplied Dynamics API params: ${result.updated}/${result.scanned} workflows`,
      );
      return;
    }
    await workflows().deleteMany({
      apiProjectId: String(existing._id),
      kind: "apiEndpoint",
    });
    const { deleteApiEndpointsForProject } = await import("../sync.js");
    await deleteApiEndpointsForProject(String(existing._id));
    await apiProjects().deleteOne({ _id: existing._id });
    console.log("Reseeding Dynamics API mirror…");
  }

  const now = new Date().toISOString();
  const project: Stored<ApiProjectDoc> = {
    _id: new ObjectId(),
    name: "Sasfin Dynamics Api",
    slug: DYNAMICS_SLUG,
    description:
      "Flow-backed mirror of Sasfin.Dynamics.Api — X-Api-Key auth, CRM Web API nodes.",
    version: "v1",
    enabled: true,
    auth: {
      mode: "none",
      jwtSecret: config.dynamicsApiKey,
      schemeName: "ApiKey",
    },
    schemas: {
      ...dynamicsSchemas,
      __seedVersion: { type: "integer", const: DYNAMICS_SEED_VERSION },
    },
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
  await apiProjects().insertOne(project);
  const projectId = String(project._id);

  for (const def of DYNAMICS_ENDPOINTS) {
    const flow = buildDynamicsFlow(def, projectId);
    await workflows().insertOne(flow);
    await syncApiEndpointsFromWorkflow({
      _id: String(flow._id),
      nodes: flow.nodes,
      enabled: flow.enabled,
      kind: flow.kind,
      apiProjectId: flow.apiProjectId,
    });
  }

  console.log(
    `Seeded Dynamics API mirror: /apis/${DYNAMICS_SLUG} (${DYNAMICS_ENDPOINTS.length} endpoints)`,
  );
}
