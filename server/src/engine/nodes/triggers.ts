import type { NodeHandler } from "../types.js";
import { normalizeItems } from "../items.js";

function asItems(
  runInput: unknown,
  items: ReturnType<typeof normalizeItems>,
  fallback: Record<string, unknown>,
) {
  if (items.length) return items;
  return normalizeItems(runInput ?? fallback);
}

export const rabbitmqTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "rabbitmq",
    queue: String(node.parameters.queue ?? ""),
  }),
});

export const redisTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "redis",
    channel: String(node.parameters.channel ?? ""),
  }),
});

export const outlookTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "outlook",
    event: String(node.parameters.event ?? "messageReceived"),
  }),
});

export const teamsTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "teams",
    event: String(node.parameters.event ?? "newChannelMessage"),
  }),
});

export const whatsappTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "whatsapp",
    event: String(node.parameters.event ?? "messages"),
  }),
});

export const formTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "form", submitted: true }),
});

export const errorTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "error" }),
});

export const localFileTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "localFile",
    path: String(node.parameters.path ?? ""),
  }),
});

export const sseTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "sse" }),
});

export const rssFeedTriggerNode: NodeHandler = async ({ items, runInput, node }) => ({
  items: asItems(runInput, items, {
    trigger: "rss",
    url: String(node.parameters.url ?? ""),
  }),
});

export const emailTriggerImapNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "email" }),
});

export const executeWorkflowTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "executeWorkflow" }),
});

export const workflowTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "workflow" }),
});

export const n8nTriggerNode: NodeHandler = async ({ items, runInput }) => ({
  items: asItems(runInput, items, { trigger: "n8n" }),
});

export const intervalNode: NodeHandler = async ({ items, runInput, node }) => {
  const intervalMinutes = Number(
    node.parameters.intervalMinutes ??
      (node.parameters.seconds != null
        ? Number(node.parameters.seconds) / 60
        : 1),
  );
  return {
    items: asItems(runInput, items, {
      trigger: "interval",
      intervalMinutes: Number.isFinite(intervalMinutes) ? intervalMinutes : 1,
    }),
  };
};

export const activationTriggerNode: NodeHandler = async ({
  items,
  runInput,
  node,
}) => ({
  items: asItems(runInput, items, {
    trigger: "activation",
    event: String(node.parameters.event ?? "activate"),
  }),
});

export const simulateTriggerNode: NodeHandler = async ({
  items,
  runInput,
  node,
}) => {
  const output = node.parameters.output;
  if (Array.isArray(output) && output.length) {
    return { items: normalizeItems(output) };
  }
  return {
    items: asItems(runInput, items, { simulated: true }),
  };
};

export const mcpServerTriggerNode: NodeHandler = async ({
  items,
  runInput,
  node,
}) => ({
  items: asItems(runInput, items, {
    trigger: "mcpServer",
    toolName: String(node.parameters.toolName ?? "run"),
  }),
});

export const evaluationTriggerNode: NodeHandler = async ({
  items,
  runInput,
  node,
}) => ({
  items: asItems(runInput, items, {
    trigger: "evaluation",
    datasetId: String(node.parameters.datasetId ?? ""),
  }),
});
