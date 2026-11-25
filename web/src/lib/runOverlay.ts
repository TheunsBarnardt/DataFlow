import type { NodeRunLog, RunStatus } from "@dataflow/shared";

export type NodeExecutionOverlay = {
  status: RunStatus;
  durationMs: number;
  itemCount: number;
};

/** Milliseconds between startedAt and finishedAt (0 if missing). */
export function logDurationMs(log: Pick<NodeRunLog, "startedAt" | "finishedAt">): number {
  if (!log.startedAt || !log.finishedAt) return 0;
  return Math.max(
    0,
    new Date(log.finishedAt).getTime() - new Date(log.startedAt).getTime(),
  );
}

export function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 10_000) return `${(ms / 1000).toFixed(2)}s`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function countArrayish(value: unknown): number | null {
  if (Array.isArray(value)) return value.length;
  return null;
}

/** Best-effort item count from a node log output payload. */
export function itemCountFromOutput(
  output: unknown,
  sourceHandle?: string | null,
): number {
  if (output == null) return 0;
  if (typeof output !== "object") return 1;

  const o = output as Record<string, unknown>;
  if (o.skipped) return 0;

  const handle = sourceHandle && sourceHandle.length ? sourceHandle : "main";

  if (o.outputs && typeof o.outputs === "object" && o.outputs) {
    const outs = o.outputs as Record<string, unknown>;
    const branch = outs[handle];
    const n = countArrayish(branch);
    if (n != null) return n;
    if (handle !== "main") {
      const main = countArrayish(outs.main);
      if (main != null) return main;
    }
  }

  const items = countArrayish(o.items);
  if (items != null) return items;

  const direct = countArrayish(output);
  if (direct != null) return direct;

  return 1;
}

export function buildNodeOverlayMap(
  logs: NodeRunLog[] | undefined | null,
): Map<string, NodeExecutionOverlay> {
  const map = new Map<string, NodeExecutionOverlay>();
  if (!logs?.length) return map;
  for (const log of logs) {
    map.set(log.nodeId, {
      status: log.status,
      durationMs: logDurationMs(log),
      itemCount: itemCountFromOutput(log.output),
    });
  }
  return map;
}

export function edgeItemCount(
  logsByNode: Map<string, NodeExecutionOverlay>,
  sourceId: string,
  sourceHandle?: string | null,
  sourceLog?: NodeRunLog,
): number | undefined {
  if (!sourceLog) {
    const overlay = logsByNode.get(sourceId);
    return overlay?.itemCount;
  }
  return itemCountFromOutput(sourceLog.output, sourceHandle);
}
