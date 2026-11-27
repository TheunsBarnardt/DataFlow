#!/usr/bin/env node
import { MongoClient } from "mongodb";

const uri = process.env.MONGODB_URI || "mongodb://127.0.0.1:27017/botflow?replicaSet=rs0";

async function main() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db();
  const projects = await db
    .collection("api_projects")
    .find({})
    .project({ slug: 1, name: 1 })
    .toArray();
  console.log(
    "projects",
    projects.map((p) => ({ id: String(p._id), slug: p.slug })),
  );
  const ids = projects.map((p) => String(p._id));
  const orphanFilter = {
    kind: "apiEndpoint",
    apiProjectId: { $nin: ids },
  };
  const orphans = await db.collection("workflows").countDocuments(orphanFilter);
  console.log("orphan apiEndpoint workflows", orphans);
  if (orphans > 0) {
    const res = await db.collection("workflows").deleteMany(orphanFilter);
    console.log("deleted orphan workflows", res.deletedCount);
    await db.collection("api_endpoints").deleteMany({
      apiProjectId: { $nin: ids },
    });
  }

  const online = await db
    .collection("workflows")
    .find({ name: /GetNotificationsAsync/, apiProjectId: { $in: ids } })
    .toArray();
  for (const w of online) {
    const mid = w.nodes[1];
    console.log({
      name: w.name,
      project: w.apiProjectId,
      type: mid?.type,
      cred: mid?.parameters?.credentialId,
      q: String(mid?.parameters?.query || "").slice(0, 100),
    });
  }

  const country = await db
    .collection("workflows")
    .find({ name: "GET ReferenceData/Country", apiProjectId: { $in: ids } })
    .toArray();
  for (const w of country) {
    const mid = w.nodes.find((n) => n.type === "dynamics") || w.nodes[1];
    console.log({
      name: w.name,
      cred: mid?.parameters?.credentialId,
      type: mid?.parameters?.type,
      op: mid?.parameters?.operation,
      fetch: String(mid?.parameters?.getQuery || "").includes("<fetch"),
    });
  }

  await client.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
