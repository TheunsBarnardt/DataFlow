use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Card, CardContent, CardHeader,
    CardTitle,
};
use crate::roles::ROLE_DEFINITIONS;

#[component]
pub fn RolesPanel() -> Element {
    let mut users = use_signal(Vec::<serde_json::Value>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            match crate::api::get_json("/api/users").await {
                Ok(v) => {
                    users.set(
                        v.get("users")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let counts: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for u in users().iter() {
            let role = u
                .get("role")
                .and_then(|x| x.as_str())
                .unwrap_or("editor");
            *map.entry(role.to_string()).or_insert(0) += 1;
        }
        map
    };

    let role_ids: String = ROLE_DEFINITIONS
        .iter()
        .map(|r| r.id)
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        div { class: "flex h-full flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25] tracking-normal", "Roles" }
                p { class: "mt-3 text-sm leading-[1.35] text-muted-foreground",
                    "Roles allow you to manage specific permissions tailored to your team's needs. Define granular access to workflows, credentials, and instance settings. Assign roles under People & access → Users."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-8 mt-2".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            div { class: "grid gap-4 p-6 md:grid-cols-2 xl:grid-cols-3",
                for role_def in ROLE_DEFINITIONS.iter() {
                    {
                        let count = counts.get(role_def.id).copied().unwrap_or(0);
                        let count_label = if loading() {
                            "…".to_string()
                        } else {
                            format!("{count} user{}", if count == 1 { "" } else { "s" })
                        };
                        rsx! {
                            Card {
                                CardHeader { class: Some("flex-row items-start justify-between gap-2 space-y-0 px-4 pt-4 pb-0".into()),
                                    div {
                                        CardTitle { class: Some("text-sm".into()), "{role_def.name}" }
                                        code { class: "text-[11px] text-muted-foreground", "{role_def.id}" }
                                    }
                                    Badge {
                                        variant: BadgeVariant::Secondary,
                                        class: Some("shrink-0".into()),
                                        "{count_label}"
                                    }
                                }
                                CardContent { class: Some("space-y-3 px-4 pb-4 pt-2".into()),
                                    p { class: "text-sm text-muted-foreground", "{role_def.description}" }
                                    ul { class: "space-y-1.5 border-t pt-3",
                                        for cap in role_def.capabilities.iter() {
                                            li { class: "flex items-center gap-2 text-[13px] text-foreground",
                                                span { class: "h-1.5 w-1.5 shrink-0 rounded-full bg-primary" }
                                                "{cap.label}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "border-t px-6 py-4 text-xs text-muted-foreground",
                "Assign roles under "
                span { class: "font-medium text-foreground", "People & access → Users" }
                " ({role_ids})."
            }
        }
    }
}