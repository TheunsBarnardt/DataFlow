import type {
  InsightsByTime,
  InsightsByWorkflow,
  InsightsSummary,
  RunDoc,
} from "@dataflow/shared";
import { runs } from "./db.js";

function periodBounds(days: number): {
  from: Date;
  to: Date;
  prevFrom: Date;
  prevTo: Date;
} {
  const to = new Date();
  const from = new Date(to.getTime() - days * 24 * 60 * 60 * 1000);
  const prevTo = from;
  const prevFrom = new Date(from.getTime() - days * 24 * 60 * 60 * 1000);
  return { from, to, prevFrom, prevTo };
}

function dayKey(iso: string): string {
  return iso.slice(0, 10);
}

function summarize(list: RunDoc[]): {
  total: number;
  failed: number;
  failureRate: number;
  timeSavedMinutes: number;
  averageRunTimeMs: number;
} {
  const total = list.length;
  const failed = list.filter((r) => r.status === "failed").length;
  const failureRate = total ? failed / total : 0;
  const timeSavedMinutes = list.reduce(
    (sum, r) => sum + (Number(r.timeSavedMinutes) || 0),
    0,
  );
  const withDuration = list.filter((r) => r.startedAt && r.finishedAt);
  const averageRunTimeMs = withDuration.length
    ? withDuration.reduce((sum, r) => {
        return (
          sum +
          (new Date(r.finishedAt!).getTime() - new Date(r.startedAt).getTime())
        );
      }, 0) / withDuration.length
    : 0;
  return { total, failed, failureRate, timeSavedMinutes, averageRunTimeMs };
}

function buildByTime(
  list: RunDoc[],
  from: Date,
  to: Date,
): InsightsByTime[] {
  const buckets = new Map<string, RunDoc[]>();
  const cursor = new Date(
    Date.UTC(from.getUTCFullYear(), from.getUTCMonth(), from.getUTCDate()),
  );
  const end = new Date(
    Date.UTC(to.getUTCFullYear(), to.getUTCMonth(), to.getUTCDate()),
  );
  while (cursor <= end) {
    buckets.set(cursor.toISOString().slice(0, 10), []);
    cursor.setUTCDate(cursor.getUTCDate() + 1);
  }
  for (const r of list) {
    const key = dayKey(r.startedAt);
    const bucket = buckets.get(key);
    if (bucket) bucket.push(r);
  }
  return [...buckets.entries()].map(([date, dayRuns]) => {
    const s = summarize(dayRuns);
    return {
      date,
      total: s.total,
      failed: s.failed,
      failureRate: s.failureRate,
      timeSavedMinutes: s.timeSavedMinutes,
      averageRunTimeMs: s.averageRunTimeMs,
    };
  });
}

export async function getInsights(days = 7): Promise<InsightsSummary> {
  const { from, to, prevFrom, prevTo } = periodBounds(Math.max(1, days));
  const allCurrent = (await runs()
    .find({ startedAt: { $gte: from.toISOString(), $lte: to.toISOString() } })
    .toArray()) as unknown as RunDoc[];

  const previous = (await runs()
    .find({
      startedAt: { $gte: prevFrom.toISOString(), $lt: prevTo.toISOString() },
    })
    .toArray()) as unknown as RunDoc[];

  const cur = summarize(allCurrent);
  const prev = summarize(previous);

  const byMap = new Map<string, RunDoc[]>();
  for (const r of allCurrent) {
    const key = r.workflowId;
    const bucket = byMap.get(key) ?? [];
    bucket.push(r);
    byMap.set(key, bucket);
  }

  const byWorkflow: InsightsByWorkflow[] = [...byMap.entries()]
    .map(([workflowId, list]) => {
      const s = summarize(list);
      return {
        workflowId,
        workflowName: list[0]?.workflowName ?? workflowId,
        total: s.total,
        failed: s.failed,
        failureRate: s.failureRate,
        timeSavedMinutes: s.timeSavedMinutes,
        averageRunTimeMs: s.averageRunTimeMs,
      };
    })
    .sort((a, b) => b.total - a.total);

  return {
    from: from.toISOString(),
    to: to.toISOString(),
    total: { value: cur.total, previous: prev.total, unit: "count" },
    failed: { value: cur.failed, previous: prev.failed, unit: "count" },
    failureRate: {
      value: cur.failureRate,
      previous: prev.failureRate,
      unit: "ratio",
    },
    timeSaved: {
      value: cur.timeSavedMinutes,
      previous: prev.timeSavedMinutes,
      unit: "minute",
    },
    averageRunTime: {
      value: cur.averageRunTimeMs,
      previous: prev.averageRunTimeMs,
      unit: "millisecond",
    },
    byWorkflow,
    byTime: buildByTime(allCurrent, from, to),
  };
}
