import { ObjectId } from "mongodb";
import type { ApiProjectDoc } from "@dataflow/shared";
import { apiProjects, workflows, type Stored } from "../../db.js";
import { config } from "../../config.js";
import { syncApiEndpointsFromWorkflow } from "../sync.js";
import { reapplyEndpointParameters } from "../reapplyParams.js";
import { ONLINE_ENDPOINTS, onlinePath } from "./catalog.js";
import { onlineSchemas } from "./schemas.js";
import { buildOnlineFlow } from "./flows.js";

const ONLINE_SLUG = "wealth-online";
/** Bump to force reseed after catalog/flow changes. */
const ONLINE_SEED_VERSION = 2;

export async function ensureWealthOnlineApiMirror(): Promise<void> {
  const existing = await apiProjects().findOne({ slug: ONLINE_SLUG });
  if (existing) {
    const wfs = await workflows()
      .find({ apiProjectId: String(existing._id), kind: "apiEndpoint" })
      .toArray();
    const seedMeta = (
      existing.schemas as Record<string, { const?: unknown }> | undefined
    )?.__seedVersion;
    const seededVersion = Number(seedMeta?.const ?? seedMeta ?? 0);
    const needsReseed =
      seededVersion !== ONLINE_SEED_VERSION ||
      wfs.length !== ONLINE_ENDPOINTS.length;
    if (!needsReseed) {
      const result = await reapplyEndpointParameters({
        apiProjectId: String(existing._id),
        project: "wealthOnline",
        endpoints: ONLINE_ENDPOINTS,
        pathOf: (d) => onlinePath(d as (typeof ONLINE_ENDPOINTS)[number]),
      });
      console.log(
        `Reapplied Wealth Online API params: ${result.updated}/${result.scanned} workflows`,
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
    console.log("Reseeding Wealth Online API mirror…");
  }

  const now = new Date().toISOString();
  const project: Stored<ApiProjectDoc> = {
    _id: new ObjectId(),
    name: "Sasfin Wealth Online Api",
    slug: ONLINE_SLUG,
    description:
      "Flow-backed mirror of Sasfin.Wealth.Online.Api — JWT auth, Dynamics.Api + MSSQL nodes.",
    version: "v1",
    enabled: true,
    auth: {
      mode: "jwt",
      jwtSecret: config.wealthOnlineJwtSecret,
      schemeName: "Bearer",
    },
    schemas: {
      ...onlineSchemas,
      __seedVersion: { type: "integer", const: ONLINE_SEED_VERSION },
    },
    createdBy: "system",
    updatedBy: "system",
    createdAt: now,
    updatedAt: now,
  };
  await apiProjects().insertOne(project);
  const projectId = String(project._id);

  for (const def of ONLINE_ENDPOINTS) {
    const flow = buildOnlineFlow(
      def,
      projectId,
      config.wealthOnlineJwtSecret,
    );
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
    `Seeded Wealth Online API mirror: /apis/${ONLINE_SLUG} (${ONLINE_ENDPOINTS.length} endpoints)`,
  );
}
