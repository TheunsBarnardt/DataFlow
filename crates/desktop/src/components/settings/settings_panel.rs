use dioxus::prelude::*;

use super::authentication_panel::AuthenticationPanel;
use super::companies_panel::CompaniesPanel;
use super::cutover_panel::CutoverPanel;
use super::dataflow_api_panel::DataFlowApiPanel;
use super::profile_panel::ProfilePanel;
use super::roles_panel::RolesPanel;
use super::settings_nav::SettingsPageId;
use super::users_panel::UsersPanel;

#[component]
fn StubPage(title: &'static str, description: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "{title}" }
                p { class: "mt-1 text-sm leading-[1.35] text-muted-foreground", "{description}" }
            }
            div { class: "flex min-h-0 flex-1 p-6",
                div { class: "flex w-full flex-1 items-center justify-center rounded-lg border border-dashed p-6 text-sm text-muted-foreground",
                    "Coming soon."
                }
            }
        }
    }
}

#[component]
pub fn SettingsPanel(
    page: SettingsPageId,
    is_admin: bool,
    #[allow(unused_variables)] on_navigate: Option<EventHandler<SettingsPageId>>,
) -> Element {
    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-hidden",
            if page == SettingsPageId::Profile {
                ProfilePanel {}
            } else if page == SettingsPageId::Users && is_admin {
                UsersPanel {}
            } else if page == SettingsPageId::Roles {
                RolesPanel {}
            } else if page == SettingsPageId::Companies && is_admin {
                CompaniesPanel {}
            } else if page == SettingsPageId::Authentication {
                AuthenticationPanel {}
            } else if page == SettingsPageId::Cutover && is_admin {
                CutoverPanel {}
            } else if page == SettingsPageId::Mcp {
                StubPage {
                    title: "Instance-level MCP",
                    description: "Expose BotFlow tools over MCP for agents and IDE clients.",
                }
            } else if page == SettingsPageId::Assistant {
                StubPage {
                    title: "Assistant",
                    description: "Configure the in-app workflow assistant.",
                }
            } else if page == SettingsPageId::Agents {
                StubPage {
                    title: "Agents",
                    description: "Preview settings for autonomous agent runs.",
                }
            } else if page == SettingsPageId::Context {
                StubPage {
                    title: "Context",
                    description: "Preview controls for shared agent context.",
                }
            } else if page == SettingsPageId::Chat {
                StubPage {
                    title: "Chat",
                    description: "Chat surface settings for assistants and agents.",
                }
            } else if page == SettingsPageId::Api {
                DataFlowApiPanel {}
            } else if page == SettingsPageId::Policies {
                StubPage {
                    title: "Policies",
                    description: "Security policies for credentials, sharing, and execution.",
                }
            } else {
                ProfilePanel {}
            }
        }
    }
}