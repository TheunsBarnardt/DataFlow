export type SettingsPageId =
  | "profile"
  | "users"
  | "roles"
  | "authentication"
  | "mcp"
  | "assistant"
  | "agents"
  | "context"
  | "chat"
  | "api"
  | "community-nodes"
  | "policies";

export type SettingsNavItem = {
  id: SettingsPageId;
  label: string;
  badge?: string;
  adminOnly?: boolean;
};

export type SettingsNavGroup = {
  label?: string;
  items: SettingsNavItem[];
};

export const SETTINGS_NAV: SettingsNavGroup[] = [
  {
    label: "Personal",
    items: [{ id: "profile", label: "Profile & preferences" }],
  },
  {
    label: "People & access",
    items: [
      { id: "users", label: "Users", adminOnly: true },
      { id: "roles", label: "Roles", adminOnly: true },
    ],
  },
  {
    items: [{ id: "authentication", label: "Authentication", adminOnly: true }],
  },
  {
    label: "AI",
    items: [
      { id: "mcp", label: "Instance-level MCP" },
      { id: "assistant", label: "Assistant" },
      { id: "agents", label: "Agents", badge: "Preview" },
      { id: "context", label: "Context", badge: "Preview" },
      { id: "chat", label: "Chat" },
    ],
  },
  {
    label: "Integrations",
    items: [
      { id: "api", label: "DataFlow API" },
      { id: "community-nodes", label: "Community nodes" },
    ],
  },
  {
    label: "Security",
    items: [{ id: "policies", label: "Policies", adminOnly: true }],
  },
];

export function visibleSettingsNav(isAdmin: boolean): SettingsNavGroup[] {
  return SETTINGS_NAV.map((group) => ({
    ...group,
    items: group.items.filter((item) => !item.adminOnly || isAdmin),
  })).filter((group) => group.items.length > 0);
}
