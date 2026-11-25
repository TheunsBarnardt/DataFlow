import { ObjectId } from "mongodb";
import type { WorkflowDoc, ScheduleDoc } from "@dataflow/shared";
import { schedules, type Stored } from "../db.js";
import { computeNext, refreshSchedules } from "../scheduler.js";

/** Upsert ScheduleDoc rows from canvas schedule + interval nodes. */
export async function syncSchedulesFromWorkflow(
  workflow: Pick<WorkflowDoc, "_id" | "nodes" | "enabled" | "name"> & {
    _id: string;
  },
): Promise<void> {
  const workflowId = String(workflow._id);
  const nodes = workflow.nodes.filter(
    (n) => n.type === "schedule" || n.type === "interval",
  );
  const keepNodeIds = new Set<string>();

  for (const node of nodes) {
    keepNodeIds.add(node.id);
    const cron =
      node.type === "schedule"
        ? String(node.parameters.cron ?? "").trim() || undefined
        : undefined;
    let intervalMinutes = Number(
      node.parameters.intervalMinutes ??
        (node.type === "interval"
          ? Number(node.parameters.seconds ?? 60) / 60
          : 5),
    );
    if (!Number.isFinite(intervalMinutes) || intervalMinutes <= 0) {
      intervalMinutes = node.type === "interval" ? 1 : 5;
    }
    // Prefer cron when set on schedule nodes
    const useCron = Boolean(cron);
    const paused = !workflow.enabled;

    const existing = await schedules().findOne({ workflowId, nodeId: node.id });
    const nextRunAt = computeNext(
      useCron ? cron : undefined,
      useCron ? undefined : intervalMinutes,
    );
    const $set: Partial<ScheduleDoc> = {
      name: `${workflow.name ?? "Workflow"} · ${node.name}`,
      cron: useCron ? cron : undefined,
      intervalMinutes: useCron ? undefined : intervalMinutes,
      paused,
      nextRunAt,
    };

    if (existing) {
      await schedules().updateOne(
        { _id: existing._id },
        {
          $set: {
            name: $set.name,
            cron: $set.cron,
            intervalMinutes: $set.intervalMinutes,
            paused: $set.paused,
            nextRunAt: $set.nextRunAt,
          },
        },
      );
    } else {
      const doc: Stored<ScheduleDoc> = {
        _id: new ObjectId(),
        workflowId,
        nodeId: node.id,
        name: $set.name!,
        cron: $set.cron,
        intervalMinutes: $set.intervalMinutes,
        paused,
        nextRunAt,
      };
      await schedules().insertOne(doc);
    }
  }

  // Drop orphaned synced schedules for this workflow (keep manual ScheduleDocs without nodeId)
  const orphans = await schedules()
    .find({ workflowId, nodeId: { $exists: true } })
    .toArray();
  for (const s of orphans) {
    if (s.nodeId && !keepNodeIds.has(s.nodeId)) {
      await schedules().deleteOne({ _id: s._id });
    }
  }

  await refreshSchedules();
}

/** Remove synced schedules when a workflow is deleted. */
export async function deleteSchedulesForWorkflow(
  workflowId: string,
): Promise<void> {
  await schedules().deleteMany({ workflowId });
  await refreshSchedules();
}
