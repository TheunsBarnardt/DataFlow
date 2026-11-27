#!/usr/bin/env node
/**
 * Insert api_endpoints rows from existing apiEndpoint workflows.
 * The studio lists /api/api-projects/:id endpoints — not raw workflows.
 */
import { MongoClient, ObjectId } from "mongodb";

const uri = process.env.MONGODB_URI || "mongodb://127.0.0.1:27017/botflow";

function oid(id) {
  if (!id) return null;
  if (typeof id === "object" && id.$oid) return new ObjectId(id.$oid);
  if (id instanceof ObjectId) return id;
  try {
    return new ObjectId(String(id));
  } catch {
    return String(id);
  }
}

function idStr(id) {
  if (!id) return "";
  if (typeof id === "object" && id.$oid) return id.$oid;
  return String(id);
}

async function main() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db();
  const workflows = db.collection("workflows");
  const endpoints = db.collection("api_endpoints");

  const cursor = workflows.find({ kind: "apiEndpoint", apiProjectId: { $exists: true, $ne: null } });
  let upserted = 0;
  let scanned = 0;

  while (await cursor.hasNext()) {
    const wf = await cursor.next();
    scanned++;
    const projectId = idStr(wf.apiProjectId);
    const workflowId = idStr(wf._id);
    const nodes = wf.nodes || [];
    const respond = nodes.find((n) => n.type === "apiRespond");

    for (const node of nodes.filter((n) => n.type === "apiEndpoint")) {
      const path = String(node.parameters?.path || "")
        .trim()
        .replace(/^\/+|\/+$/g, "");
      if (!path) continue;
      let method = String(node.parameters?.method || "GET").toUpperCase();
      if (!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"].includes(method)) {
        method = "GET";
      }
      let auth = String(node.parameters?.auth || "none");
      if (!["jwt", "headerBearer", "apiKey"].includes(auth)) auth = "none";
      const summary =
        node.parameters?.summary || node.name || `${method} ${path}`;
      const tag =
        node.parameters?.tag ||
        path.split("/")[0] ||
        "default";
      const operationId =
        node.parameters?.operationId ||
        `${method}_${path.replace(/[^a-zA-Z0-9]/g, "_")}`;
      const responseSchema =
        respond?.parameters?.responseSchema ||
        node.parameters?.responseSchema || {
          type: "object",
          properties: {},
          required: [],
        };
      const responseStatus =
        respond?.parameters?.statusCode ??
        node.parameters?.responseStatus ??
        200;

      const doc = {
        apiProjectId: projectId,
        workflowId,
        nodeId: node.id,
        path,
        method,
        summary,
        tag,
        auth,
        operationId,
        responseStatus: Number(responseStatus) || 200,
        responseSchema,
        requestQuerySchema: node.parameters?.requestQuerySchema,
        requestBodySchema: node.parameters?.requestBodySchema,
      };

      await endpoints.updateOne(
        { apiProjectId: projectId, method, path },
        { $set: doc, $setOnInsert: { _id: new ObjectId() } },
        { upsert: true },
      );
      upserted++;
    }
  }

  const byProject = await endpoints
    .aggregate([{ $group: { _id: "$apiProjectId", n: { $sum: 1 } } }])
    .toArray();
  console.log(`Scanned ${scanned} workflows, upserted ${upserted} endpoint rows`);
  console.log("Per project:", byProject);
  await client.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
