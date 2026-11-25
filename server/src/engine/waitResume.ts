import { ObjectId } from "mongodb";
import type { WorkflowItem } from "./items.js";
import { runs, toId } from "../db.js";
import { nanoid } from "nanoid";

export type PendingWait = {
  runId: string;
  workflowId: string;
  nodeId: string;
  resumeToken: string;
  items: WorkflowItem[];
  /** Serialized executor continuation is too heavy — resume re-runs from wait node output */
  resumeAt?: string;
  timer?: NodeJS.Timeout;
};

const pending = new Map<string, PendingWait>();

export function registerWait(wait: Omit<PendingWait, "timer">): PendingWait {
  const prev = pending.get(wait.runId);
  if (prev?.timer) clearTimeout(prev.timer);
  const entry: PendingWait = { ...wait };
  if (wait.resumeAt) {
    const ms = new Date(wait.resumeAt).getTime() - Date.now();
    if (ms > 0 && ms < 24 * 60 * 60_000) {
      entry.timer = setTimeout(() => {
        void resumeWait(wait.runId, wait.resumeToken, { reason: "timer" });
      }, ms);
    }
  }
  pending.set(wait.runId, entry);
  return entry;
}

export function getWait(runId: string): PendingWait | undefined {
  return pending.get(runId);
}

export async function resumeWait(
  runId: string,
  token: string,
  input?: unknown,
): Promise<{ ok: boolean; error?: string; runId?: string }> {
  const entry = pending.get(runId);
  if (!entry) return { ok: false, error: "No pending wait for this run" };
  if (entry.resumeToken !== token) {
    return { ok: false, error: "Invalid resume token" };
  }
  if (entry.timer) clearTimeout(entry.timer);
  pending.delete(runId);

  const { resumeWorkflowAfterWait } = await import("./executor.js");
  const run = await resumeWorkflowAfterWait({
    runId: entry.runId,
    workflowId: entry.workflowId,
    nodeId: entry.nodeId,
    items: entry.items,
    resumeInput: input,
  });
  return { ok: true, runId: run._id };
}

export function createResumeToken(): string {
  return nanoid(24);
}

export async function markRunWaiting(
  runId: string | ObjectId,
  details: {
    nodeId: string;
    resumeToken: string;
    resumeAt?: string;
    resumeUrl?: string;
  },
): Promise<void> {
  await runs().updateOne(
    { _id: toId(String(runId)) },
    {
      $set: {
        status: "waiting",
        wait: details,
      },
    },
  );
}
