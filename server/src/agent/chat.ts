import { nanoid } from "nanoid";
import type { ChatMessage } from "@dataflow/shared";
import { threads, toId, serialize } from "../db.js";
import { agentTools, callAgentTool } from "./tools.js";

function extractJsonBlock(text: string): unknown | null {
  const fence = text.match(/```json\s*([\s\S]*?)```/i);
  if (fence) {
    try {
      return JSON.parse(fence[1]);
    } catch {
      /* fall through */
    }
  }
  const bare = text.trim();
  if (bare.startsWith("{") || bare.startsWith("[")) {
    try {
      return JSON.parse(bare);
    } catch {
      return null;
    }
  }
  return null;
}

async function interpretAndAct(content: string): Promise<{
  reply: string;
  toolCalls: { name: string; result: unknown }[];
}> {
  const toolCalls: { name: string; result: unknown }[] = [];
  const lower = content.toLowerCase();

  // Structured tool call from Cursor/Claude-style JSON
  const parsed = extractJsonBlock(content) as
    | { tool?: string; arguments?: Record<string, unknown>; name?: string; nodes?: unknown[] }
    | null;

  if (parsed?.tool) {
    const result = await callAgentTool(
      parsed.tool,
      parsed.arguments ?? {},
    );
    toolCalls.push({ name: parsed.tool, result });
    return {
      reply: result.ok
        ? `Ran \`${parsed.tool}\` successfully.\n\`\`\`json\n${JSON.stringify(result.data, null, 2)}\n\`\`\``
        : `Tool \`${parsed.tool}\` failed: ${result.error}`,
      toolCalls,
    };
  }

  if (parsed?.name && Array.isArray(parsed.nodes)) {
    const result = await callAgentTool("create_workflow", parsed as Record<string, unknown>);
    toolCalls.push({ name: "create_workflow", result });
    return {
      reply: result.ok
        ? `Created workflow **${parsed.name}**. Open it on the canvas.\n\`\`\`json\n${JSON.stringify(result.data, null, 2)}\n\`\`\``
        : `Failed to create workflow: ${result.error}`,
      toolCalls,
    };
  }

  if (lower.includes("list workflow") || lower.trim() === "list" || lower.includes("show flows")) {
    const result = await callAgentTool("list_workflows", {});
    toolCalls.push({ name: "list_workflows", result });
    const list = (result.data as { name: string; _id: string }[]) ?? [];
    return {
      reply:
        list.length === 0
          ? "No workflows yet. Ask me to create one, or paste a JSON workflow."
          : list.map((w) => `- **${w.name}** (\`${w._id}\`)`).join("\n"),
      toolCalls,
    };
  }

  // Natural language: create a simple webhook -> http or set demo flow
  if (lower.includes("create") && (lower.includes("flow") || lower.includes("workflow") || lower.includes("webhook"))) {
    const wantsHttp = lower.includes("http");
    const wantsMssql = lower.includes("mssql") || lower.includes("sql");
    const nodes = [
      {
        id: "n1",
        type: "webhook",
        name: "Webhook",
        position: { x: 80, y: 120 },
        parameters: {},
      },
      wantsMssql
        ? {
            id: "n2",
            type: "mssql",
            name: "MSSQL",
            position: { x: 320, y: 120 },
            parameters: { query: "SELECT 1 AS ok" },
          }
        : wantsHttp
          ? {
              id: "n2",
              type: "httpRequest",
              name: "HTTP Request",
              position: { x: 320, y: 120 },
              parameters: {
                url: "https://httpbin.org/get",
                method: "GET",
              },
            }
          : {
              id: "n2",
              type: "set",
              name: "Set fields",
              position: { x: 320, y: 120 },
              parameters: { values: { hello: "DataFlow", ok: true } },
            },
      {
        id: "n3",
        type: "webhookRespond",
        name: "Respond",
        position: { x: 560, y: 120 },
        parameters: { statusCode: 200 },
      },
    ];
    const create = await callAgentTool("create_workflow", {
      name: wantsMssql
        ? "Webhook → MSSQL"
        : wantsHttp
          ? "Webhook → HTTP"
          : "Webhook → Set",
      description: "Created from chat",
      nodes,
    });
    toolCalls.push({ name: "create_workflow", result: create });
    if (!create.ok) {
      return { reply: `Failed: ${create.error}`, toolCalls };
    }
    const wf = create.data as { _id: string };
    const hook = await callAgentTool("create_webhook", {
      workflowId: wf._id,
    });
    toolCalls.push({ name: "create_webhook", result: hook });
    return {
      reply: `Created workflow and webhook.\n\`\`\`json\n${JSON.stringify({ workflow: create.data, webhook: hook.data }, null, 2)}\n\`\`\`\nOpen the canvas to inspect the flow.`,
      toolCalls,
    };
  }

  if (lower.startsWith("run ") || lower.includes("run workflow")) {
    const idMatch = content.match(/[a-f0-9]{24}/i);
    if (idMatch) {
      const result = await callAgentTool("run_workflow", {
        workflowId: idMatch[0],
      });
      toolCalls.push({ name: "run_workflow", result });
      return {
        reply: result.ok
          ? `Run finished.\n\`\`\`json\n${JSON.stringify(result.data, null, 2)}\n\`\`\``
          : `Run failed: ${result.error}`,
        toolCalls,
      };
    }
  }

  const toolNames = agentTools.map((t) => t.name).join(", ");
  return {
    reply: [
      "I'm the DataFlow agent entrypoint (local, no paid API keys).",
      "Cursor or Claude should drive me via MCP for full LLM planning.",
      "",
      "In this chat you can:",
      "- Say **create webhook flow** / **create http flow** / **create mssql flow**",
      "- Say **list workflows**",
      "- Say **run \<workflowId\>**",
      "- Paste a JSON workflow `{ \"name\", \"nodes\", \"edges\" }`",
      `- Or a tool call \`\`\`json { \"tool\": \"create_workflow\", \"arguments\": { ... } } \`\`\``,
      "",
      `Tools: ${toolNames}`,
    ].join("\n"),
    toolCalls,
  };
}

export async function handleAgentMessage(threadId: string, content: string) {
  const thread = await threads().findOne({ _id: toId(threadId) });
  if (!thread) throw new Error("Thread not found");

  const userMsg: ChatMessage = {
    id: nanoid(),
    role: "user",
    content,
    createdAt: new Date().toISOString(),
  };

  const { reply, toolCalls } = await interpretAndAct(content);

  const assistantMsg: ChatMessage = {
    id: nanoid(),
    role: "assistant",
    content: reply,
    createdAt: new Date().toISOString(),
  };

  const toolMsgs: ChatMessage[] = toolCalls.map((tc) => ({
    id: nanoid(),
    role: "tool" as const,
    toolName: tc.name,
    content: JSON.stringify(tc.result),
    createdAt: new Date().toISOString(),
  }));

  const messages = [...thread.messages, userMsg, ...toolMsgs, assistantMsg];
  const title =
    thread.messages.length === 0
      ? content.slice(0, 48) || "New chat"
      : thread.title;

  await threads().updateOne(
    { _id: thread._id },
    {
      $set: {
        messages,
        title,
        updatedAt: new Date().toISOString(),
      },
    },
  );

  const updated = await threads().findOne({ _id: thread._id });
  return {
    thread: serialize(updated!),
    reply: assistantMsg,
    toolCalls,
  };
}
