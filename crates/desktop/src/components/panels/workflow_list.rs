use dioxus::prelude::*;

use crate::components::icons::{Folder, MoreHorizontal, Pencil, Plus, Search, Trash2};
use crate::components::panels::{format_local_datetime, json_bool, json_id, json_string};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, Card, ConfirmDeleteDialog, Empty,
    EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle, Input, PageLoader,
    Switch,
};

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
    let mut menu_for = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);
    let mut editing = use_signal(|| None::<(String, String)>);
    let mut busy = use_signal(|| false);

    let create_busy = creating || creating_local();

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

    let reload = move || {
        spawn(async move {
            if let Ok(v) = crate::api::get_json("/api/workflows").await {
                workflows.set(
                    v.get("workflows")
                        .and_then(|w| w.as_array())
                        .cloned()
                        .unwrap_or_default(),
                );
            }
        });
    };

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
                        Input {
                            class: Some("w-56 pl-8".into()),
                            placeholder: Some("Search workflows…".into()),
                            value: query(),
                            oninput: move |e: FormEvent| query.set(e.value()),
                        }
                    }
                    Button {
                        disabled: create_busy,
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
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("shrink-0 rounded-none border-x-0 border-t-0 border-b".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            if loading() {
                PageLoader { label: "Loading projects…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    Card {
                        class: Some(if menu_for().is_some() {
                            "relative z-50 overflow-visible p-0 gap-0".into()
                        } else {
                            "overflow-hidden p-0 gap-0".into()
                        }),
                        div {
                            class: "flex w-full items-center gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                            div { class: "min-w-0 flex-1", "Name" }
                            div { class: "shrink-0", style: "width:100px", "Status" }
                            div { class: "shrink-0", style: "width:120px", "Version" }
                            div { class: "shrink-0", style: "width:140px", "Updated" }
                            div { class: "w-10 shrink-0" }
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
                                let id_menu = id.clone();
                                let id_ctx = id.clone();
                                let menu_open = menu_for().as_deref() == Some(id.as_str());
                                let row_class = if menu_open {
                                    "relative z-10 flex w-full items-center gap-3 border-b px-4 py-3 text-sm last:border-b-0 hover:bg-muted/50"
                                } else {
                                    "relative flex w-full items-center gap-3 border-b px-4 py-3 text-sm last:border-b-0 hover:bg-muted/50"
                                };
                                let edit_id = id.clone();
                                let edit_name = name.clone();
                                let del_id = id.clone();
                                let del_name = name.clone();
                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "{row_class}",
                                        oncontextmenu: move |e| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            menu_for.set(Some(id_ctx.clone()));
                                        },
                                        button {
                                            class: "min-w-0 flex-1 truncate text-left font-medium",
                                            onclick: move |_| on_open.call(id_open.clone()),
                                            "{name}"
                                        }
                                        div { class: "shrink-0", style: "width:100px",
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
                                        div { class: "shrink-0 text-muted-foreground", style: "width:120px", "v{version}" }
                                        div { class: "shrink-0 truncate text-xs text-muted-foreground", style: "width:140px", "{updated}" }
                                        div { class: "relative flex w-10 shrink-0 justify-end",
                                            button {
                                                r#type: "button",
                                                class: "rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground",
                                                title: "More",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    if menu_for().as_deref() == Some(id_menu.as_str()) {
                                                        menu_for.set(None);
                                                    } else {
                                                        menu_for.set(Some(id_menu.clone()));
                                                    }
                                                },
                                                MoreHorizontal { class: Some("h-4 w-4".into()) }
                                            }
                                            if menu_open {
                                                div {
                                                    class: "absolute right-0 top-full z-50 mt-1 min-w-[10rem] rounded-md border bg-popover p-1 text-popover-foreground text-sm shadow-md",
                                                    onclick: move |e| e.stop_propagation(),
                                                    button {
                                                        r#type: "button",
                                                        class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                                        onclick: move |_| {
                                                            menu_for.set(None);
                                                            editing.set(Some((edit_id.clone(), edit_name.clone())));
                                                        },
                                                        Pencil { class: Some("h-4 w-4".into()) }
                                                        "Edit"
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-destructive hover:bg-destructive/10",
                                                        onclick: move |_| {
                                                            menu_for.set(None);
                                                            confirm_delete.set(Some((del_id.clone(), del_name.clone())));
                                                        },
                                                        Trash2 { class: Some("h-4 w-4".into()) }
                                                        "Delete"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if filtered.is_empty() {
                            Empty { class: Some("border-0 py-12".into()),
                                EmptyHeader {
                                    EmptyMedia { variant: EmptyMediaVariant::Icon,
                                        Folder { class: Some("size-5".into()) }
                                    }
                                    EmptyTitle { class: Some("text-sm".into()), "No workflows yet" }
                                    EmptyDescription { "Create one to open the editor." }
                                }
                            }
                        }
                    }
                }
            }
            }

            if menu_for().is_some() {
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| menu_for.set(None),
                }
            }

            if let Some((edit_id, edit_name)) = editing() {
                {
                    let edit_id_save = edit_id.clone();
                    rsx! {
                        div {
                            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4 backdrop-blur-[1px]",
                            onclick: move |_| editing.set(None),
                            div {
                                class: "w-full max-w-sm rounded-xl border bg-card p-5 shadow-2xl",
                                onclick: move |e| e.stop_propagation(),
                                role: "dialog",
                                "aria-modal": "true",
                                h2 { class: "text-lg font-semibold", "Edit project" }
                                p { class: "mt-2 text-sm text-muted-foreground", "Change the workflow name." }
                                Input {
                                    class: Some("mt-4".into()),
                                    value: edit_name.clone(),
                                    oninput: move |e: FormEvent| {
                                        editing.set(Some((edit_id.clone(), e.value())));
                                    },
                                }
                                div { class: "mt-5 flex justify-end gap-2",
                                    Button {
                                        variant: crate::components::ui::ButtonVariant::Ghost,
                                        disabled: busy(),
                                        onclick: move |_| editing.set(None),
                                        "Cancel"
                                    }
                                    Button {
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let id = edit_id_save.clone();
                                            let name = editing()
                                                .map(|(_, n)| n.trim().to_string())
                                                .unwrap_or_default();
                                            if name.is_empty() {
                                                return;
                                            }
                                            busy.set(true);
                                            spawn(async move {
                                                match crate::api::put_json(
                                                    &format!("/api/workflows/{id}"),
                                                    serde_json::json!({ "name": name }),
                                                )
                                                .await
                                                {
                                                    Ok(_) => {
                                                        editing.set(None);
                                                        err.set(None);
                                                        reload();
                                                    }
                                                    Err(e) => err.set(Some(e)),
                                                }
                                                busy.set(false);
                                            });
                                        },
                                        "Save"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some((del_id, del_name)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let description = format!(
                        "This will permanently delete \"{del_name}\" and its history. This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete project?".to_string(),
                            description,
                            busy: busy(),
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                let id = del_id_confirm.clone();
                                busy.set(true);
                                spawn(async move {
                                    match crate::api::delete_json(&format!("/api/workflows/{id}")).await {
                                        Ok(_) => {
                                            confirm_delete.set(None);
                                            err.set(None);
                                            reload();
                                        }
                                        Err(e) => err.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}
