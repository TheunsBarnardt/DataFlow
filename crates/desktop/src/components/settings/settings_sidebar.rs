use dioxus::prelude::*;

use crate::components::icons::ChevronLeft;
use crate::components::settings::settings_nav::{SettingsNavGroup, SettingsPageId, visible_settings_nav};
use crate::components::ui::{Badge, BadgeVariant};

#[component]
pub fn SettingsSidebar(
    page: SettingsPageId,
    is_admin: bool,
    on_select: EventHandler<SettingsPageId>,
    on_back: EventHandler<()>,
) -> Element {
    let nav = visible_settings_nav(is_admin);

    rsx! {
        aside { class: "flex h-full w-[200px] shrink-0 flex-col border-r bg-[var(--sidebar)]",
            div { class: "flex items-center gap-1 border-b px-1.5 py-1.5",
                button {
                    class: "flex h-8 w-full items-center gap-1 rounded-md px-1 text-sm font-medium hover:bg-muted",
                    title: "Back",
                    onclick: move |_| on_back.call(()),
                    span { class: "grid h-5 w-5 place-items-center",
                        ChevronLeft { class: Some("h-4 w-4".into()) }
                    }
                    "Settings"
                }
            }
            nav { class: "flex flex-1 flex-col gap-4 overflow-y-auto px-1.5 pt-3",
                for (gi, group) in nav.iter().enumerate() {
                    SettingsNavGroupView {
                        key: "{gi}",
                        group: group.clone(),
                        page,
                        on_select,
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsNavGroupView(
    group: SettingsNavGroup,
    page: SettingsPageId,
    on_select: EventHandler<SettingsPageId>,
) -> Element {
    rsx! {
        div { class: "flex flex-col",
            if let Some(label) = group.label {
                div { class: "mb-0.5 px-1.5 py-0.5 text-xs font-medium text-muted-foreground", "{label}" }
            }
            for item in group.items {
                {
                    let active = page == item.id;
                    let id = item.id;
                    rsx! {
                        button {
                            key: "{item.label}",
                            class: if active {
                                "flex w-full items-center justify-between gap-2 rounded-md bg-accent px-1.5 py-1.5 text-left text-sm font-medium text-foreground"
                            } else {
                                "flex w-full items-center justify-between gap-2 rounded-md px-1.5 py-1.5 text-left text-sm text-foreground/80 hover:bg-muted hover:text-foreground"
                            },
                            onclick: move |_| on_select.call(id),
                            span { class: "truncate", "{item.label}" }
                            if let Some(badge) = item.badge {
                                Badge {
                                    variant: BadgeVariant::Secondary,
                                    class: Some("h-5 shrink-0 px-1.5 text-[10px] font-medium".into()),
                                    "{badge}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}