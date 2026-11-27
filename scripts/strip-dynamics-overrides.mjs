#!/usr/bin/env node
/** Remove baseUrl / accessToken overrides from all dynamics nodes (use credential instead). */
import { MongoClient } from "mongodb";

const uri = process.env.MONGODB_URI || "mongodb://127.0.0.1:27017/botflow";

async function main() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db();
  const workflows = db.collection("workflows");

  const creds = await db
    .collection("credentials")
    .find({})
    .project({ name: 1, type: 1 })
    .toArray();
  const dataverse = creds.find(
    (c) =>
      /dataverse|dynamics/i.test(c.type || "") ||
      /dataverse|dynamics|uat/i.test(c.name || ""),
  );
  console.log(
    "Dataverse credential:",
    dataverse
      ? { id: String(dataverse._id), name: dataverse.name, type: dataverse.type }
      : "(none found — leave credentialId empty)",
  );

  const cursor = workflows.find({ "nodes.type": "dynamics" });
  let modified = 0;
  let scanned = 0;
  while (await cursor.hasNext()) {
    const wf = await cursor.next();
    scanned++;
    let changed = false;
    const nodes = (wf.nodes || []).map((n) => {
      if (n.type !== "dynamics") return n;
      const p = { ...(n.parameters || {}) };
      if (p.baseUrl || p.accessToken) {
        p.baseUrl = "";
        p.accessToken = "";
        changed = true;
      }
      if (dataverse && !p.credentialId) {
        p.credentialId = String(dataverse._id);
        changed = true;
      }
      return { ...n, parameters: p };
    });
    if (changed) {
      await workflows.updateOne({ _id: wf._id }, { $set: { nodes } });
      modified++;
    }
  }
  console.log(`Scanned ${scanned} workflows, updated ${modified}`);

  const sample = await workflows.findOne({
    "nodes.type": "dynamics",
    name: /ReferenceData\/Country/,
  });
  const crm = (sample?.nodes || []).find((n) => n.type === "dynamics");
  console.log("Country params:", JSON.stringify(crm?.parameters, null, 2));
  await client.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
