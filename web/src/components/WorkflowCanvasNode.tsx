import { memo, type MouseEvent } from "react";
import {
  Handle,
  Position,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { Check, Loader2, Plus, X } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  addFromHandleTooltip,
  sourceHandleLabel,
  sourceHandlesForType,
  targetHandlesForType,
  targetHandleLabel,
} from "@/lib/nodeHandles";
import { getNodeColor, getNodeIcon, isTriggerType } from "@/lib/nodeIcons";
import { formatDurationMs } from "@/lib/runOverlay";

export type WorkflowCanvasNodeData = {
  name: string;
  type: string;
  disabled?: boolean;
  note?: string;
  /** Switch case / rule output labels (source handles). */
  switchCases?: string[];
  errorOutput?: boolean;
  runStatus?: "completed" | "failed" | "running" | string;
  durationMs?: number;
  /** Sticky note background color. */
  color?: string;
  onEdit?: (id: string) => void;
  onExecute?: (id: string) => void;
  onToggleDisabled?: (id: string) => void;
  onAddFromHandle?: (id: string, sourceHandle: string) => void;
};

export type WorkflowCanvasFlowNode = Node<
  WorkflowCanvasNodeData,
  "workflow"
>;

function WorkflowCanvasNodeComponent({
  id,
  data,
  selected,
}: NodeProps<WorkflowCanvasFlowNode>) {
  const isSticky = data.type === "stickyNote";
  const isTrigger = isTriggerType(data.type);
  const sources = sourceHandlesForType(data.type, data.switchCases, {
    errorOutput: data.errorOutput,
  });
  const targets = targetHandlesForType(data.type);
  const multiOut =
    sources.length > 1 || (sources[0] !== "main" && sources[0] !== undefined);
  const multiIn = targets.length > 1;

  const accent = getNodeColor(data.type);
  const Icon = getNodeIcon(data.type);
  const status = data.runStatus;
  const showDuration =
    typeof data.durationMs === "number" &&
    data.durationMs >= 0 &&
    (status === "completed" || status === "failed");

  function handleAddFrom(e: MouseEvent, handle: string) {
    e.stopPropagation();
    data.onAddFromHandle?.(id, handle);
  }

  if (isSticky) {
    const stickyBg = data.color || "#fef3c7";
    const content = (data.note || data.name || "").trim() || "Sticky note";
    return (
      <div
        className={cn(
          "relative min-h-[100px] min-w-[160px] max-w-[280px] rounded-lg border border-black/10 px-3 py-2 shadow-md",
          selected && "ring-2 ring-[var(--brand)] ring-offset-2 ring-offset-[var(--canvas)]",
          data.disabled && "opacity-55",
        )}
        style={{ background: stickyBg }}
      >
        <div className="whitespace-pre-wrap break-words text-sm leading-snug text-foreground/90">
          {content}
        </div>
      </div>
    );
  }

  // Node bounds = icon box so React Flow anchors handles on the square edges
  // (label hangs below and must not widen the measured node).
  return (
    <div
      className={cn(
        "group relative size-[72px]",
        data.disabled && "opacity-55",
      )}
    >
      {targets.map((hid, i) => (
        <Handle
          key={`t-${hid}`}
          id={hid}
          type="target"
          position={Position.Left}
          style={
            multiIn
              ? {
                  top: `${((i + 1) / (targets.length + 1)) * 100}%`,
                }
              : undefined
          }
          className="!size-[11px] !border-2 !border-[var(--canvas)] !bg-muted-foreground !shadow-sm"
          title={targetHandleLabel(data.type, hid)}
        />
      ))}

      <div
        className={cn(
          "relative flex size-[72px] items-center justify-center border-[2.5px] bg-card shadow-[0_1px_3px_rgba(0,0,0,0.12),0_4px_12px_rgba(0,0,0,0.06)] transition-[box-shadow,transform] duration-150",
          isTrigger
            ? "rounded-[28px_12px_12px_28px]"
            : "rounded-xl",
          selected &&
            "shadow-[0_0_0_4px_color-mix(in_oklch,var(--brand)_32%,transparent),0_4px_16px_rgba(0,0,0,0.14)]",
          status === "completed"
            ? "ring-2 ring-emerald-500/90 ring-offset-2 ring-offset-[var(--canvas)]"
            : status === "failed"
              ? "ring-2 ring-red-500/90 ring-offset-2 ring-offset-[var(--canvas)]"
              : status === "running"
                ? "ring-2 ring-[var(--brand)] ring-offset-2 ring-offset-[var(--canvas)]"
                : null,
        )}
        style={{
          borderColor: accent,
          color: accent,
          backgroundImage: `linear-gradient(160deg, color-mix(in srgb, ${accent} 14%, var(--card)) 0%, var(--card) 55%)`,
        }}
      >
        <Icon className="size-7 drop-shadow-sm" strokeWidth={2} />
      </div>

      {status === "completed" ? (
        <span className="absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-emerald-500 text-white shadow-md ring-2 ring-[var(--canvas)]">
          <Check className="size-3" strokeWidth={3} />
        </span>
      ) : status === "failed" ? (
        <span className="absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-red-500 text-white shadow-md ring-2 ring-[var(--canvas)]">
          <X className="size-3" strokeWidth={3} />
        </span>
      ) : status === "running" ? (
        <span className="absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-[var(--brand)] text-white shadow-md ring-2 ring-[var(--canvas)]">
          <Loader2 className="size-3 animate-spin" strokeWidth={3} />
        </span>
      ) : null}

      <div className="pointer-events-none absolute left-1/2 top-[calc(100%+0.5rem)] w-[148px] -translate-x-1/2 px-0.5 text-center">
        <div className="truncate text-[12px] font-semibold leading-tight tracking-tight text-foreground">
          {data.name}
        </div>
        {showDuration ? (
          <div className="mt-0.5 text-[10px] tabular-nums text-muted-foreground">
            {formatDurationMs(data.durationMs!)}
          </div>
        ) : null}
        {data.disabled ? (
          <div className="mt-0.5 text-[10px] text-muted-foreground">
            deactivated
          </div>
        ) : null}
      </div>

      <TooltipProvider delayDuration={200}>
        {sources.map((hid, i) => {
          const top = multiOut
            ? `${((i + 1) / (sources.length + 1)) * 100}%`
            : "50%";
          const isError = hid === "error";
          const tip = addFromHandleTooltip(data.type, hid);
          return (
            <div key={`s-${hid}`} className="contents">
              <Handle
                id={hid}
                type="source"
                position={Position.Right}
                style={multiOut ? { top } : undefined}
                className={cn(
                  "!size-[11px] !border-2 !border-[var(--canvas)] !shadow-sm",
                  isError ? "!bg-destructive" : "!bg-muted-foreground",
                )}
                title={sourceHandleLabel(data.type, hid)}
              />
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    className={cn(
                      "nodrag nopan absolute z-10 flex size-6 items-center justify-center rounded-full border border-border bg-card text-muted-foreground shadow-sm",
                      "opacity-0 transition-opacity duration-150 group-hover:opacity-100 hover:border-muted-foreground/60 hover:text-foreground",
                    )}
                    style={{
                      top,
                      right: -28,
                      transform: "translateY(-50%)",
                    }}
                    aria-label={tip}
                    onClick={(e) => handleAddFrom(e, hid)}
                  >
                    <Plus className="size-3" strokeWidth={2.5} />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right">{tip}</TooltipContent>
              </Tooltip>
            </div>
          );
        })}
      </TooltipProvider>

      {multiOut ? (
        <div className="pointer-events-none absolute -right-1 top-[calc(100%+0.35rem)] flex flex-col items-end gap-0.5 text-[8px] leading-none text-muted-foreground">
          {sources.map((h) => (
            <span key={h} className={h === "error" ? "text-destructive" : undefined}>
              {sourceHandleLabel(data.type, h)}
            </span>
          ))}
        </div>
      ) : null}
      {multiIn ? (
        <div className="pointer-events-none absolute -left-1 top-[calc(100%+0.35rem)] flex flex-col gap-0.5 text-[8px] leading-none text-muted-foreground">
          {targets.map((h) => (
            <span key={h}>{targetHandleLabel(data.type, h)}</span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export const WorkflowCanvasNode = memo(WorkflowCanvasNodeComponent);
