export const DATAFLOW_ROLES = [
  "admin",
  "editor",
  "viewer",
  "api_consumer",
  "mcp_consumer",
] as const;

export type DataFlowRole = (typeof DATAFLOW_ROLES)[number];

export type RoleCapability = {
  id: string;
  label: string;
};

export type RoleDefinition = {
  id: DataFlowRole;
  name: string;
  description: string;
  capabilities: RoleCapability[];
};

export const ROLE_DEFINITIONS: RoleDefinition[] = [
  {
    id: "admin",
    name: "Admin",
    description: "Full instance control, including users and settings.",
    capabilities: [
      { id: "settings", label: "Manage settings & users" },
      { id: "workflows_write", label: "Create and edit workflows" },
      { id: "credentials", label: "Manage credentials" },
      { id: "execute", label: "Execute workflows" },
      { id: "api", label: "Call API projects" },
      { id: "mcp", label: "Use MCP tools" },
    ],
  },
  {
    id: "editor",
    name: "Editor",
    description: "Build and run workflows; cannot manage users or roles.",
    capabilities: [
      { id: "workflows_write", label: "Create and edit workflows" },
      { id: "credentials", label: "Manage credentials" },
      { id: "execute", label: "Execute workflows" },
      { id: "api", label: "Call API projects" },
    ],
  },
  {
    id: "viewer",
    name: "Viewer",
    description: "Read-only access to workflows, runs, and insights.",
    capabilities: [
      { id: "workflows_read", label: "View workflows" },
      { id: "runs_read", label: "View executions & insights" },
    ],
  },
  {
    id: "api_consumer",
    name: "API consumer",
    description: "Call published API project endpoints with scoped keys.",
    capabilities: [
      { id: "api", label: "Call API projects" },
    ],
  },
  {
    id: "mcp_consumer",
    name: "MCP consumer",
    description: "Invoke instance MCP tools from external agents/clients.",
    capabilities: [
      { id: "mcp", label: "Use MCP tools" },
    ],
  },
];
