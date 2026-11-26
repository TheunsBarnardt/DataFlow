use dioxus::prelude::*;

use crate::components::icons::{Plus, Search};
use crate::components::panels::{format_local_datetime, json_bool, json_id, json_string};
use crate::components::ui::{Button, PageLoader, Switch};

#[component]
pub fn WorkflowList(
    on_open: EventHandler<String>,
    on_create: EventHandler<()>,
    on_toggle_enabled: EventHandler<(String, bool)>,
    #[props(default)] refresh_key: u32,
    #[props(default)] creating: bool,
    /// When true, hide apiEndpoint workflows (Projects list). When false and
    /// `api_only` is true, show only api endpoints.
    #[props(default)]
    exclude_api: bool,
    #[props(default)]
    api_only: bool,
) -> Element {
    let mut workflows = use_signal(Vec::<serde_json::Value>::new);
    let mut query = use_signal(String::new);
    let mut err = use_signal(|| None::<String>);
    let mut creating_local = use_signal(|| false);
    let mut loading = use_signal(|| true);

    let busy = creating || creating_local();

    use_effect(move || {
        let _ = refresh_key;
        loading.set(true);
        spawn(async move {
            match crate::api::get_json("/api/workflows").await {
                Ok(v) => {
                    let list = v
                        .get("workflows")
                        .and_then(|w| w.as_array())
                        .cloned()
                        .unwrap_or_default();
                    workflows.set(list);
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let q = query().trim().to_lowercase();
    let filtered: Vec<serde_json::Value> = workflows()
        .into_iter()
        .filter(|w| {
            let kind = w.get("kind").map(json_string).unwrap_or_default();
            let is_api = kind == "apiEndpoint" || w.get("apiProjectId").is_some();
            if exclude_api && is_api {
                return false;
            }
            if api_only && !is_api {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            let name = w.get("name").map(json_string).unwrap_or_default();
            name.to_lowercase().contains(&q)
        })
        .collect();

    rsx! {
        div { class: "flex h-full flex-col",
            div { class: "flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5",
                div {
                    h1 { class: "text-2xl font-semibold tracking-tight", "Projects" }
                    p { class: "mt-1 text-sm text-muted-foreground",
                        "Your workflows and projects"
                    }
                }
                div { class: "flex items-center gap-2",
                    div { class: "relative",
                        Search { class: Some("pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground".into()) }
                        input {
                            class: "flex h-9 w-56 rounded-md border border-input bg-transparent px-3 py-1 pl-8 text-sm shadow-sm",
                            placeholder: "Search workflows…",
                            value: "{query()}",
                            oninput: move |e| query.set(e.value()),
                        }
                    }
                    Button {
                        disabled: busy,
                        onclick: move |_| {
                            creating_local.set(true);
                            spawn(async move {
                                let body = serde_json::json!({ "name": "New workflow" });
                                let res = crate::api::post_json("/api/workflows", body).await;
                                creating_local.set(false);
                                match res {
                                    Ok(v) => {
                                        if let Some(id) = v.get("workflow").map(json_id).filter(|s| !s.is_empty()) {
                                            on_create.call(());
                                            on_open.call(id);
                                        } else {
                                            on_create.call(());
                                        }
                                    }
                                    Err(e) => err.set(Some(e)),
                                }
                            });
                        },
                        Plus { class: Some("h-4 w-4".into()) }
                        "Create workflow"
                    }
                }
            }

            if let Some(message) = err() {
                div { class: "border-b bg-destructive/10 px-6 py-2 text-sm text-destructive", "{message}" }
            }

            if loading() {
                PageLoader { label: "Loading projects…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    div { class: "overflow-hidden rounded-lg border bg-card",
                        div { class: "grid grid-cols-[1.5fr_100px_120px_140px] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                            div { "Name" }
                            div { "Status" }
                            div { "Version" }
                            div { "Updated" }
                        }
                        for w in filtered.iter().cloned() {
                            {
                                let id = json_id(&w);
                                let name = w.get("name").map(json_string).unwrap_or_else(|| "Untitled".into());
                                let enabled = json_bool(&w, "enabled", false);
                                let version = w.get("version").map(json_string).unwrap_or_else(|| "1".into());
                                let updated = format_local_datetime(
                                    &w.get("updatedAt").map(json_string).unwrap_or_default(),
                                );
                                let id_open = id.clone();
                                let id_toggle = id.clone();
                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "grid w-full grid-cols-[1.5fr_100px_120px_140px] items-center gap-3 border-b px-4 py-3 text-sm last:border-b-0 hover:bg-muted/50",
                                        button {
                                            class: "truncate text-left font-medium",
                                            onclick: move |_| on_open.call(id_open.clone()),
                                            "{name}"
                                        }
                                        div {
                                            Switch {
                                                checked: enabled,
                                                size: Some("sm".into()),
                                                on_checked_change: move |checked| {
                                                    let body = serde_json::json!({ "enabled": checked });
                                                    let id_for_api = id_toggle.clone();
                                                    let path = format!("/api/workflows/{id_for_api}");
                                                    spawn(async move {
                                                        if crate::api::put_json(&path, body).await.is_ok() {
                                                            on_toggle_enabled.call((id_for_api, checked));
                                                        }
                                                    });
                                                },
                                            }
                                        }
                                        div { class: "text-muted-foreground", "v{version}" }
                                        div { class: "truncate text-xs text-muted-foreground", "{updated}" }
                                    }
                                }
                            }
                        }
                        if filtered.is_empty() {
                            div { class: "px-4 py-12 text-center text-sm text-muted-foreground",
                                "No workflows yet. Create one to open the editor."
                            }
                        }
                    }
                }
            }
            }
        }
    }
}