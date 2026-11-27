use dioxus::prelude::*;

use crate::components::icons::{
    Boxes, Chart, Folder, Globe, Key, LayoutDashboard, LogOut, Moon, Play, Plus, ScrollText,
    Settings, Sun, Variable, ChevronLeft, ChevronRight,
};
use crate::components::settings::{visible_settings_nav, SettingsPageId};
use crate::components::ui::{
    Avatar, AvatarFallback, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Separator,
};
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NavSection {
    #[default]
    Dashboard,
    Workflows,
    Apis,
    Variables,
    Credentials,
    Nodes,
    Runs,
    Insights,
    Audit,
    Settings,
}

#[component]
pub fn MainSidebar(
    section: NavSection,
    collapsed: bool,
    theme: Theme,
    actor_name: String,
    #[props(default = "admin".to_string())] role: String,
    on_navigate: EventHandler<NavSection>,
    on_new_workflow: EventHandler<()>,
    on_toggle_collapse: EventHandler<()>,
    on_toggle_theme: EventHandler<()>,
    on_profile: EventHandler<()>,
    on_open_settings: EventHandler<SettingsPageId>,
    on_logout: EventHandler<()>,
) -> Element {
    let mut settings_open = use_signal(|| false);
    let is_admin = role == "admin";
    let settings_nav = visible_settings_nav(is_admin);
    let display_role = if role.is_empty() {
        "user".to_string()
    } else {
        role.clone()
    };

    let initials = {
        let parts: Vec<_> = actor_name.split_whitespace().collect();
        let s: String = parts.iter().filter_map(|p| p.chars().next()).take(2).collect();
        if s.is_empty() {
            "DF".into()
        } else {
            s.to_uppercase()
        }
    };

    let width = if collapsed { "w-[52px]" } else { "w-[220px]" };

    rsx! {
        aside {
            class: "flex h-full shrink-0 flex-col border-r bg-[var(--sidebar)] transition-[width] duration-200 {width}",
            div {
                class: if collapsed {
                    "flex flex-col items-center gap-2 border-b px-3 py-3"
                } else {
                    "flex items-center justify-between border-b px-3 py-3"
                },
                div {
                    class: if collapsed { "flex flex-col items-center gap-2" } else { "flex items-center gap-2" },
                    div {
                        class: "flex h-7 w-7 items-center justify-center rounded-md text-[11px] font-bold text-white",
                        style: "background: var(--brand)",
                        title: "DataFlow",
                        "DF"
                    }
                    if !collapsed {
                        span { class: "text-sm font-semibold tracking-tight", "DataFlow" }
                    }
                }
                Button {
                    size: ButtonSize::IconSm,
                    variant: ButtonVariant::Outline,
                    title: Some("New workflow".into()),
                    onclick: move |_| on_new_workflow.call(()),
                    Plus { class: Some("h-4 w-4".into()) }
                }
            }

            div { class: "flex flex-1 flex-col gap-4 overflow-y-auto p-2",
                div { class: "flex flex-col gap-0.5",
                    NavBtn { id: NavSection::Dashboard, label: "Overview", section, collapsed, on_navigate,
                        icon: rsx! { LayoutDashboard { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Workflows, label: "Projects", section, collapsed, on_navigate,
                        icon: rsx! { Folder { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Apis, label: "API Projects", section, collapsed, on_navigate,
                        icon: rsx! { Globe { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    if !collapsed {
                        div { class: "mt-3 px-2.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground",
                            "Projects"
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            class: Some("mt-1 h-auto w-full justify-start gap-2 px-2.5 py-2 text-[13px] text-muted-foreground".into()),
                            onclick: move |_| on_new_workflow.call(()),
                            Plus { class: Some("h-3.5 w-3.5".into()) }
                            "Add project"
                        }
                    }
                }

                div { class: "mt-auto flex flex-col gap-0.5 border-t pt-2",
                    NavBtn { id: NavSection::Variables, label: "Variables", section, collapsed, on_navigate,
                        icon: rsx! { Variable { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Credentials, label: "Credentials", section, collapsed, on_navigate,
                        icon: rsx! { Key { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Nodes, label: "Nodes", section, collapsed, on_navigate,
                        icon: rsx! { Boxes { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Runs, label: "Executions", section, collapsed, on_navigate,
                        icon: rsx! { Play { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Insights, label: "Insights", section, collapsed, on_navigate,
                        icon: rsx! { Chart { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                    NavBtn { id: NavSection::Audit, label: "Audit", section, collapsed, on_navigate,
                        icon: rsx! { ScrollText { class: Some("h-4 w-4 shrink-0".into()) } }
                    }
                }
            }

            div { class: "space-y-1 border-t p-2",
                Button {
                    variant: ButtonVariant::Ghost,
                    class: Some(if collapsed {
                        "h-auto w-full justify-center gap-2.5 px-0 py-2 text-[13px] text-muted-foreground"
                    } else {
                        "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-[13px] text-muted-foreground"
                    }.into()),
                    title: Some(if theme == Theme::Dark { "Light mode" } else { "Dark mode" }.into()),
                    onclick: move |_| on_toggle_theme.call(()),
                    if theme == Theme::Dark {
                        Sun { class: Some("h-4 w-4".into()) }
                    } else {
                        Moon { class: Some("h-4 w-4".into()) }
                    }
                    if !collapsed {
                        if theme == Theme::Dark { "Light mode" } else { "Dark mode" }
                    }
                }
                div { class: "relative",
                    {
                        let active = section == NavSection::Settings || settings_open();
                        let settings_class = if collapsed {
                            if active {
                                "h-auto w-full justify-center gap-2.5 px-0 py-2 text-[13px] bg-accent font-medium text-foreground"
                            } else {
                                "h-auto w-full justify-center gap-2.5 px-0 py-2 text-[13px] text-muted-foreground"
                            }
                        } else if active {
                            "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-[13px] bg-accent font-medium text-foreground"
                        } else {
                            "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-[13px] text-muted-foreground"
                        };
                        rsx! {
                            Button {
                                variant: if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                                class: Some(settings_class.into()),
                                title: Some("Settings".into()),
                                onclick: move |_| settings_open.with_mut(|v| *v = !*v),
                                Settings { class: Some("h-4 w-4".into()) }
                                if !collapsed { "Settings" }
                            }
                        }
                    }
                    if settings_open() {
                        div {
                            class: "absolute bottom-0 left-full z-50 ml-2 max-h-[min(80vh,720px)] w-[260px] overflow-y-auto rounded-md border bg-popover p-2 text-popover-foreground shadow-md",
                            for (gi, group) in settings_nav.iter().enumerate() {
                                {
                                    let items = group.items.clone();
                                    let label = group.label;
                                    rsx! {
                                        div {
                                            key: "{label.unwrap_or(\"g\")}-{gi}",
                                            if gi > 0 {
                                                Separator { class: Some("my-2".into()) }
                                            }
                                            if let Some(label) = label {
                                                div { class: "px-2.5 py-1.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground",
                                                    "{label}"
                                                }
                                            }
                                            for item in items.into_iter() {
                                                {
                                                    let page = item.id;
                                                    let item_label = item.label;
                                                    let badge = item.badge;
                                                    rsx! {
                                                        Button {
                                                            key: "{item_label}",
                                                            variant: ButtonVariant::Ghost,
                                                            class: Some("h-auto w-full justify-between gap-2 rounded-sm px-2.5 py-2 text-left text-[13px]".into()),
                                                            onclick: move |_| {
                                                                settings_open.set(false);
                                                                on_open_settings.call(page);
                                                            },
                                                            span { class: "truncate", "{item_label}" }
                                                            if let Some(b) = badge {
                                                                Badge {
                                                                    variant: BadgeVariant::Secondary,
                                                                    class: Some("h-5 shrink-0 px-1.5 text-[10px] font-medium".into()),
                                                                    "{b}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Separator { class: Some("my-1".into()) }
                Button {
                    variant: ButtonVariant::Ghost,
                    class: Some(if collapsed {
                        "h-auto w-full justify-center gap-2 px-0 py-2 text-left"
                    } else {
                        "h-auto w-full justify-start gap-2 px-2.5 py-2 text-left"
                    }.into()),
                    title: Some("Profile".into()),
                    onclick: move |_| on_profile.call(()),
                    Avatar {
                        class: Some("size-7 after:border-transparent".into()),
                        AvatarFallback {
                            class: Some("bg-[var(--brand)] text-[10px] font-semibold text-white".into()),
                            "{initials}"
                        }
                    }
                    if !collapsed {
                        div { class: "min-w-0",
                            div { class: "truncate text-[13px] font-medium", "{actor_name}" }
                            div { class: "text-[10px] text-muted-foreground", "{display_role}" }
                            div { class: "text-[10px] text-muted-foreground/80", "v{env!(\"CARGO_PKG_VERSION\")}" }
                        }
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    class: Some(if collapsed {
                        "h-auto w-full justify-center gap-2.5 px-0 py-2 text-[13px] text-muted-foreground"
                    } else {
                        "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-[13px] text-muted-foreground"
                    }.into()),
                    title: Some("Sign out".into()),
                    onclick: move |_| on_logout.call(()),
                    LogOut { class: Some("h-4 w-4".into()) }
                    if !collapsed { "Sign out" }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    class: Some(if collapsed {
                        "h-auto w-full justify-center gap-2 px-0 py-1.5 text-[12px] text-muted-foreground"
                    } else {
                        "h-auto w-full justify-start gap-2 px-2.5 py-1.5 text-[12px] text-muted-foreground"
                    }.into()),
                    onclick: move |_| on_toggle_collapse.call(()),
                    if collapsed {
                        ChevronRight { class: Some("h-4 w-4".into()) }
                    } else {
                        ChevronLeft { class: Some("h-4 w-4".into()) }
                        "Collapse"
                    }
                }
            }
        }
    }
}

#[component]
fn NavBtn(
    id: NavSection,
    label: &'static str,
    section: NavSection,
    collapsed: bool,
    on_navigate: EventHandler<NavSection>,
    icon: Element,
) -> Element {
    let active = section == id;
    let class = if collapsed {
        if active {
            "h-auto w-full justify-center gap-2.5 px-0 py-2 text-left text-[13px] bg-accent font-medium text-foreground"
        } else {
            "h-auto w-full justify-center gap-2.5 px-0 py-2 text-left text-[13px] text-muted-foreground"
        }
    } else if active {
        "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-left text-[13px] bg-accent font-medium text-foreground"
    } else {
        "h-auto w-full justify-start gap-2.5 px-2.5 py-2 text-left text-[13px] text-muted-foreground"
    };
    rsx! {
        Button {
            variant: if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
            title: Some(label.to_string()),
            class: Some(class.into()),
            onclick: move |_| on_navigate.call(id),
            {icon}
            if !collapsed {
                span { class: "truncate", "{label}" }
            }
        }
    }
}
