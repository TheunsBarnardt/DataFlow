import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { agentTools, callAgentTool } from "../agent/tools.js";
import { DAY_ONE_NODE_TYPES } from "@dataflow/shared";

export function createMcpServer() {
  const server = new Server(
    { name: "dataflow", version: "0.1.0" },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [
      ...agentTools.map((t) => ({
        name: t.name,
        description: t.description,
        inputSchema: t.inputSchema,
      })),
      {
        name: "list_node_types",
        description: "List day-one DataFlow node types",
        inputSchema: { type: "object", properties: {} },
      },
    ],
  }));

  server.setRequestHandler(CallToolRequestSchema, async (req) => {
    const name = req.params.name;
    const args = (req.params.arguments ?? {}) as Record<string, unknown>;

    if (name === "list_node_types") {
      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(DAY_ONE_NODE_TYPES, null, 2),
          },
        ],
      };
    }

    const result = await callAgentTool(name, args);
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(result, null, 2),
        },
      ],
      isError: !result.ok,
    };
  });

  return server;
}
