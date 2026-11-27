#!/usr/bin/env node
import { MongoClient } from "mongodb";

const uri = process.env.MONGODB_URI || "mongodb://127.0.0.1:27017/botflow?replicaSet=rs0";

async function main() {
  const client = new MongoClient(uri);
  await client.connect();
  const db = client.db();
  const wf = await db.collection("workflows").findOne({
    name: "GET Admin/Notifications",
  });
  const sql = wf?.nodes?.find((n) => n.type === "mssql");
  console.log(
    JSON.stringify(
      {
        name: wf?.name,
        tags: wf?.tags,
        credentialId: sql?.parameters?.credentialId,
        queryHead: String(sql?.parameters?.query || "").slice(0, 280),
      },
      null,
      2,
    ),
  );

  const post = await db.collection("workflows").findOne({
    name: "GET Admin/PostNotificationsAsync",
  });
  const pq = post?.nodes?.find((n) => n.type === "mssql")?.parameters;
  console.log("post", {
    cred: pq?.credentialId,
    isActive: /IsActive = 1/.test(pq?.query || ""),
  });

  const pre = await db.collection("workflows").findOne({
    name: /PreNotificationsAsync/,
  });
  console.log("pre", pre?.name, pre?.nodes?.find((n) => n.type === "mssql")?.parameters?.credentialId);

  const country = await db.collection("workflows").findOne({
    name: "GET ReferenceData/Country",
  });
  const dyn = country?.nodes?.find((n) => n.type === "dynamics");
  console.log("dynamics Country", {
    cred: dyn?.parameters?.credentialId,
    type: dyn?.parameters?.type,
    fetch: String(dyn?.parameters?.getQuery || "").includes("<fetch"),
  });

  const online = await db.collection("workflows").findOne({
    name: /GetNotificationsAsync/,
  });
  console.log("online notif", {
    name: online?.name,
    mid: online?.nodes?.[1]?.type,
    cred: online?.nodes?.[1]?.parameters?.credentialId,
  });

  await client.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
