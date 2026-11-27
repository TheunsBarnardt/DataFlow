pub const DATAFLOW_ROLES: &[&str] = &[
    "admin",
    "editor",
    "viewer",
    "api_consumer",
    "mcp_consumer",
    "shell_user",
];

#[allow(dead_code)]
pub struct RoleCapability {
    pub id: &'static str,
    pub label: &'static str,
}

#[allow(dead_code)]
pub struct RoleDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub capabilities: &'static [RoleCapability],
}

#[allow(dead_code)]
pub static ROLE_DEFINITIONS: &[RoleDefinition] = &[
    RoleDefinition {
        id: "admin",
        name: "Admin",
        description: "Full instance control, including users and settings.",
        capabilities: &[
            RoleCapability {
                id: "settings",
                label: "Manage settings & users",
            },
            RoleCapability {
                id: "workflows_write",
                label: "Create and edit workflows",
            },
            RoleCapability {
                id: "credentials",
                label: "Manage credentials",
            },
            RoleCapability {
                id: "execute",
                label: "Execute workflows",
            },
            RoleCapability {
                id: "api",
                label: "Call API projects",
            },
            RoleCapability {
                id: "mcp",
                label: "Use MCP tools",
            },
        ],
    },
    RoleDefinition {
        id: "editor",
        name: "Editor",
        description: "Build and run workflows; cannot manage users or roles.",
        capabilities: &[
            RoleCapability {
                id: "workflows_write",
                label: "Create and edit workflows",
            },
            RoleCapability {
                id: "credentials",
                label: "Manage credentials",
            },
            RoleCapability {
                id: "execute",
                label: "Execute workflows",
            },
            RoleCapability {
                id: "api",
                label: "Call API projects",
            },
        ],
    },
    RoleDefinition {
        id: "viewer",
        name: "Viewer",
        description: "Read-only access to workflows, runs, and insights.",
        capabilities: &[
            RoleCapability {
                id: "workflows_read",
                label: "View workflows",
            },
            RoleCapability {
                id: "runs_read",
                label: "View executions & insights",
            },
        ],
    },
    RoleDefinition {
        id: "api_consumer",
        name: "API consumer",
        description: "Call published API project endpoints with scoped keys.",
        capabilities: &[RoleCapability {
            id: "api",
            label: "Call API projects",
        }],
    },
    RoleDefinition {
        id: "mcp_consumer",
        name: "MCP consumer",
        description: "Invoke instance MCP tools from external agents/clients.",
        capabilities: &[RoleCapability {
            id: "mcp",
            label: "Use MCP tools",
        }],
    },
    RoleDefinition {
        id: "shell_user",
        name: "Shell user",
        description: "End-user of a dumb client shell (BFF / screen APIs).",
        capabilities: &[
            RoleCapability {
                id: "bff",
                label: "Call screen / BFF endpoints",
            },
            RoleCapability {
                id: "api",
                label: "Call API projects",
            },
        ],
    },
];