#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsPageId {
    Profile,
    Users,
    Roles,
    Authentication,
    Mcp,
    Assistant,
    Agents,
    Context,
    Chat,
    Api,
    CommunityNodes,
    Policies,
}

impl SettingsPageId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Users => "users",
            Self::Roles => "roles",
            Self::Authentication => "authentication",
            Self::Mcp => "mcp",
            Self::Assistant => "assistant",
            Self::Agents => "agents",
            Self::Context => "context",
            Self::Chat => "chat",
            Self::Api => "api",
            Self::CommunityNodes => "community-nodes",
            Self::Policies => "policies",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "users" => Self::Users,
            "roles" => Self::Roles,
            "authentication" => Self::Authentication,
            "mcp" => Self::Mcp,
            "assistant" => Self::Assistant,
            "agents" => Self::Agents,
            "context" => Self::Context,
            "chat" => Self::Chat,
            "api" => Self::Api,
            "community-nodes" => Self::CommunityNodes,
            "policies" => Self::Policies,
            _ => Self::Profile,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct SettingsNavItem {
    pub id: SettingsPageId,
    pub label: &'static str,
    pub badge: Option<&'static str>,
    pub admin_only: bool,
}

#[derive(Clone, PartialEq)]
pub struct SettingsNavGroup {
    pub label: Option<&'static str>,
    pub items: Vec<SettingsNavItem>,
}

pub fn visible_settings_nav(is_admin: bool) -> Vec<SettingsNavGroup> {
    let mut groups: Vec<SettingsNavGroup> = vec![
        SettingsNavGroup {
            label: Some("Personal"),
            items: vec![SettingsNavItem {
                id: SettingsPageId::Profile,
                label: "Profile & preferences",
                badge: None,
                admin_only: false,
            }],
        },
        SettingsNavGroup {
            label: Some("People & access"),
            items: vec![
                SettingsNavItem {
                    id: SettingsPageId::Users,
                    label: "Users",
                    badge: None,
                    admin_only: true,
                },
                SettingsNavItem {
                    id: SettingsPageId::Roles,
                    label: "Roles",
                    badge: None,
                    admin_only: true,
                },
            ],
        },
        SettingsNavGroup {
            label: None,
            items: vec![SettingsNavItem {
                id: SettingsPageId::Authentication,
                label: "Authentication",
                badge: None,
                admin_only: true,
            }],
        },
        SettingsNavGroup {
            label: Some("AI"),
            items: vec![
                SettingsNavItem {
                    id: SettingsPageId::Mcp,
                    label: "Instance-level MCP",
                    badge: None,
                    admin_only: false,
                },
                SettingsNavItem {
                    id: SettingsPageId::Assistant,
                    label: "Assistant",
                    badge: None,
                    admin_only: false,
                },
                SettingsNavItem {
                    id: SettingsPageId::Agents,
                    label: "Agents",
                    badge: Some("Preview"),
                    admin_only: false,
                },
                SettingsNavItem {
                    id: SettingsPageId::Context,
                    label: "Context",
                    badge: Some("Preview"),
                    admin_only: false,
                },
                SettingsNavItem {
                    id: SettingsPageId::Chat,
                    label: "Chat",
                    badge: None,
                    admin_only: false,
                },
            ],
        },
        SettingsNavGroup {
            label: Some("Integrations"),
            items: vec![
                SettingsNavItem {
                    id: SettingsPageId::Api,
                    label: "DataFlow API",
                    badge: None,
                    admin_only: false,
                },
                SettingsNavItem {
                    id: SettingsPageId::CommunityNodes,
                    label: "Community nodes",
                    badge: None,
                    admin_only: false,
                },
            ],
        },
        SettingsNavGroup {
            label: Some("Security"),
            items: vec![SettingsNavItem {
                id: SettingsPageId::Policies,
                label: "Policies",
                badge: None,
                admin_only: true,
            }],
        },
    ];

    groups = groups
        .into_iter()
        .map(|mut g| {
            g.items.retain(|i| !i.admin_only || is_admin);
            g
        })
        .filter(|g| !g.items.is_empty())
        .collect();

    groups
}