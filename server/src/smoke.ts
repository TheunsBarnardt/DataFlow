import { connectDb, closeDb, workflows, webhooks, serialize } from "./db.js";
import { executeWorkflow } from "./engine/executor.js";
import { callAgentTool } from "./agent/tools.js";
import { itemJson } from "./engine/items.js";
import { runSmokeBasics } from "./engine/smokeBasics.js";
import { runSmokeAllNodes } from "./engine/smokeAllNodes.js";

async function main() {
  await runSmokeBasics();
  await runSmokeAllNodes();

  await connectDb();
  console.log("smoke: connected to mongo");

  const created = await callAgentTool("create_workflow", {
    name: "Smoke Transform Chain",
    nodes: [
      {
        id: "n1",
        type: "manual",
        name: "Manual",
        position: { x: 0, y: 0 },
        parameters: {},
      },
      {
        id: "n2",
        type: "set",
        name: "Set",
        position: { x: 200, y: 0 },
        parameters: {
          values: { smoke: true, label: "={{ $json.hello }}" },
          include: "all",
        },
      },
      {
        id: "n3",
        type: "filter",
        name: "Filter",
        position: { x: 400, y: 0 },
        parameters: { field: "smoke", operator: "equals", value: true },
      },
    ],
    edges: [
      { id: "e1", source: "n1", target: "n2" },
      { id: "e2", source: "n2", target: "n3" },
    ],
  });
  if (!created.ok) throw new Error(String(created.error));
  const wf = created.data as { _id: string };
  console.log("smoke: created workflow", wf._id);

  const run = await executeWorkflow({
    workflowId: wf._id,
    trigger: "manual",
    input: { hello: "world" },
  });
  console.log("smoke: run status", run.status, run.output);

  if (run.status !== "completed") {
    throw new Error("Run did not complete");
  }
  const out = Array.isArray(run.output) ? run.output : [];
  if (out.length) {
    const json = itemJson(out[0] as never);
    console.log("smoke: final item json", json);
  }

  const branched = await callAgentTool("create_workflow", {
    name: "Smoke If Branches",
    nodes: [
      {
        id: "t",
        type: "manual",
        name: "Manual",
        position: { x: 0, y: 0 },
        parameters: {},
      },
      {
        id: "i",
        type: "if",
        name: "If",
        position: { x: 200, y: 0 },
        parameters: { field: "flag", operator: "equals", value: true },
      },
      {
        id: "yes",
        type: "set",
        name: "Yes",
        position: { x: 400, y: -40 },
        parameters: { values: { path: "true" }, include: "all" },
      },
      {
        id: "no",
        type: "set",
        name: "No",
        position: { x: 400, y: 40 },
        parameters: { values: { path: "false" }, include: "all" },
      },
    ],
    edges: [
      { id: "e0", source: "t", target: "i" },
      { id: "e1", source: "i", target: "yes", sourceHandle: "true" },
      { id: "e2", source: "i", target: "no", sourceHandle: "false" },
    ],
  });
  if (!branched.ok) throw new Error(String(branched.error));
  const wf2 = branched.data as { _id: string };
  const runTrue = await executeWorkflow({
    workflowId: wf2._id,
    trigger: "manual",
    input: { flag: true },
  });
  const runFalse = await executeWorkflow({
    workflowId: wf2._id,
    trigger: "manual",
    input: { flag: false },
  });
  console.log("smoke: if true", runTrue.status, runTrue.output);
  console.log("smoke: if false", runFalse.status, runFalse.output);

  if (runTrue.status !== "completed" || runFalse.status !== "completed") {
    throw new Error("If branch runs failed");
  }

  const hook = await callAgentTool("create_webhook", { workflowId: wf._id });
  console.log("smoke: webhook", hook.data);

  const list = await workflows().find({}).toArray();
  console.log("smoke: workflows count", list.length);

  const hooks = await webhooks().find({}).toArray();
  console.log("smoke: webhooks", hooks.map((h) => serialize(h)));

  console.log("smoke: OK");
  await closeDb();
}

main().catch(async (err) => {
  console.error("smoke: FAILED", err);
  await closeDb().catch(() => undefined);
  process.exit(1);
});
