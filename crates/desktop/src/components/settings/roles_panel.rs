use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, Card, CardContent, CardHeader, CardTitle,
    Checkbox, Empty, EmptyDescription, Input, Label,
};

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn perm_ids(v: &serde_json::Value) -> Vec<String> {
    v.get("permissions")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[component]
pub fn RolesPanel() -> Element {
    let mut roles = use_signal(Vec::<serde_json::Value>::new);
    let mut catalog = use_signal(Vec::<serde_json::Value>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut new_id = use_signal(String::new);
    let mut new_name = use_signal(String::new);
    let mut new_desc = use_signal(String::new);
    let mut selected_perms = use_signal(Vec::<String>::new);
    let mut creating = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            let roles_res = crate::api::get_json("/api/roles").await;
            let perms_res = crate::api::get_json("/api/permissions").await;
            match roles_res {
                Ok(v) => {
                    roles.set(
                        v.get("roles")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            if let Ok(v) = perms_res {
                catalog.set(
                    v.get("permissions")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            loading.set(false);
        });
    });

    let create = move |_| {
        let id = new_id().trim().to_string();
        let name = new_name().trim().to_string();
        if id.is_empty() || name.is_empty() {
            err.set(Some("Role id and name required".into()));
            return;
        }
        creating.set(true);
        let desc = new_desc();
        let perms = selected_perms();
        spawn(async move {
            let body = serde_json::json!({
                "id": id,
                "name": name,
                "description": desc,
                "permissions": perms,
            });
            match crate::api::post_json("/api/roles", body).await {
                Ok(_) => {
                    new_id.set(String::new());
                    new_name.set(String::new());
                    new_desc.set(String::new());
                    selected_perms.set(Vec::new());
                    if let Ok(v) = crate::api::get_json("/api/roles").await {
                        roles.set(
                            v.get("roles")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            creating.set(false);
        });
    };

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25] tracking-normal", "Roles" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Define roles and attach permissions. Assign roles under Users. Permissions are enforced on studio and public API routes."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription { class: Some("text-destructive".into()), "{message}" }
                }
            }

            div { class: "grid min-h-0 flex-1 gap-6 p-6 lg:grid-cols-[minmax(280px,360px)_1fr]",
                Card { class: Some("self-start".into()),
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()), "Create role" }
                    }
                    CardContent { class: Some("space-y-3 px-4 pb-4".into()),
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Id" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_id(),
                                placeholder: Some("portfolio_viewer".into()),
                                oninput: move |e: FormEvent| new_id.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_name(),
                                placeholder: Some("Portfolio viewer".into()),
                                oninput: move |e: FormEvent| new_name.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Description" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_desc(),
                                oninput: move |e: FormEvent| new_desc.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Permissions" }
                            div { class: "max-h-64 space-y-0.5 overflow-y-auto rounded-md border p-1.5",
                                for p in catalog() {
                                    {
                                        let pid = json_str(&p, "id");
                                        let label = json_str(&p, "label");
                                        let checked = selected_perms().iter().any(|x| x == &pid);
                                        let pid_t = pid.clone();
                                        rsx! {
                                            label { class: "flex cursor-pointer items-start gap-2 rounded-sm px-1.5 py-1.5 text-sm hover:bg-muted/50",
                                                Checkbox {
                                                    class: Some("mt-0.5".into()),
                                                    checked,
                                                    aria_label: label.clone(),
                                                    on_checked_change: move |next| {
                                                        selected_perms.with_mut(|ids| {
                                                            if next {
                                                                if !ids.iter().any(|x| x == &pid_t) {
                                                                    ids.push(pid_t.clone());
                                                                }
                                                            } else {
                                                                ids.retain(|x| x != &pid_t);
                                                            }
                                                        });
                                                    },
                                                }
                                                span { class: "min-w-0 leading-snug",
                                                    span { class: "block", "{label}" }
                                                    code { class: "text-[11px] text-muted-foreground", "{pid}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Button { disabled: creating(), onclick: create, "Create role" }
                    }
                }

                Card { class: Some("flex min-h-0 min-w-0 flex-col overflow-hidden p-0 gap-0".into()),
                    div { class: "shrink-0 border-b bg-muted/40 px-4 py-3",
                        h2 { class: "text-sm font-semibold",
                            if loading() {
                                "Loading…"
                            } else {
                                "{roles().len()} roles"
                            }
                        }
                    }
                    if loading() {
                        CardContent { class: Some("p-4 text-sm text-muted-foreground".into()), "Loading…" }
                    } else if roles().is_empty() {
                        Empty { class: Some("border-0 py-10".into()),
                            EmptyDescription { "No roles" }
                        }
                    } else {
                        div { class: "min-h-0 flex-1 divide-y overflow-y-auto",
                            for role in roles() {
                                {
                                    let name = json_str(&role, "name");
                                    let id = json_str(&role, "id");
                                    let desc = json_str(&role, "description");
                                    let builtin = role.get("builtin").and_then(|x| x.as_bool()).unwrap_or(false);
                                    let perms = perm_ids(&role);
                                    rsx! {
                                        div { class: "space-y-2 px-4 py-3",
                                            div { class: "min-w-0",
                                                div { class: "flex flex-wrap items-center gap-2",
                                                    span { class: "text-sm font-medium", "{name}" }
                                                    if builtin {
                                                        span { class: "rounded border px-1.5 py-0.5 text-[11px] text-muted-foreground",
                                                            "Built-in"
                                                        }
                                                    }
                                                }
                                                code { class: "text-[11px] text-muted-foreground", "{id}" }
                                            }
                                            if !desc.is_empty() {
                                                p { class: "text-sm text-muted-foreground", "{desc}" }
                                            }
                                            if !perms.is_empty() {
                                                div { class: "flex flex-wrap gap-1",
                                                    for p in perms {
                                                        span { class: "rounded border px-1.5 py-0.5 text-[11px]", "{p}" }
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
}
