import cron from "node-cron";
import { CronExpressionParser } from "cron-parser";
import { ObjectId } from "mongodb";
import { schedules } from "./db.js";
import { executeWorkflow } from "./engine/executor.js";

const timers = new Map<string, cron.ScheduledTask | NodeJS.Timeout>();

export function computeNext(cronExpr?: string, intervalMinutes?: number): string {
  if (cronExpr) {
    const interval = CronExpressionParser.parse(cronExpr);
    return interval.next().toISOString() ?? new Date().toISOString();
  }
  const mins = intervalMinutes ?? 60;
  return new Date(Date.now() + mins * 60_000).toISOString();
}

async function fireSchedule(
  id: ObjectId,
  workflowId: string,
  cronExpr?: string,
  intervalMinutes?: number,
  nodeId?: string,
) {
  const trigger = cronExpr ? "schedule" : "interval";
  await executeWorkflow({
    workflowId,
    trigger,
    input: {
      scheduleId: String(id),
      nodeId,
      at: new Date().toISOString(),
      intervalMinutes,
      cron: cronExpr,
    },
  });
  const nextRunAt = computeNext(cronExpr, intervalMinutes);
  await schedules().updateOne(
    { _id: id },
    { $set: { lastRunAt: new Date().toISOString(), nextRunAt } },
  );
}

export async function refreshSchedules(): Promise<void> {
  for (const t of timers.values()) {
    if (typeof t === "object" && t && "stop" in t) {
      (t as cron.ScheduledTask).stop();
    } else {
      clearInterval(t as NodeJS.Timeout);
    }
  }
  timers.clear();

  const all = await schedules().find({ paused: false }).toArray();
  for (const s of all) {
    const id = String(s._id);
    if (s.cron && cron.validate(s.cron)) {
      const task = cron.schedule(s.cron, () => {
        void fireSchedule(
          s._id,
          s.workflowId,
          s.cron,
          s.intervalMinutes,
          s.nodeId,
        );
      });
      timers.set(id, task);
    } else if (s.intervalMinutes && s.intervalMinutes > 0) {
      const ms = Math.max(1_000, s.intervalMinutes * 60_000);
      const handle = setInterval(() => {
        void fireSchedule(
          s._id,
          s.workflowId,
          s.cron,
          s.intervalMinutes,
          s.nodeId,
        );
      }, ms);
      timers.set(id, handle);
      if (!s.nextRunAt) {
        await schedules().updateOne(
          { _id: s._id },
          { $set: { nextRunAt: computeNext(undefined, s.intervalMinutes) } },
        );
      }
    }
  }
}
