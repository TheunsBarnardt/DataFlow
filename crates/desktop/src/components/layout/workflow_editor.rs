use dioxus::prelude::*;
use dioxus_flow::prelude::{Edge, Node};

use crate::components::flow::{
    http_method_badge_class, merge_flow_into_workflow_json, PendingAddRequest, StudioNodeData,
};
use crate::components::icons::{
    Boxes, ChevronLeft, Copy, Download, FlaskConical, History, MessageSquare, MoreHorizontal,
    Pencil, Share2, Trash2, X,
};
use crate::components::panels::{
    api_endpoint_form_fields, build_run_input, json_bool, json_id, json_string, needs_execute_dialog,
    ChatPanel, ExecuteWorkflowDialog, FormField, NodeCatalogItem, NodesPanel, NodesPanelVariant,
    RunsPanel, WorkflowApiInsights,
};
use crate::components::ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, ConfirmDeleteDialog, Switch,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorTab {
    #[default]
    Editor,
    Executions,
    Insights,
}

fn split_http_method_name(name: &str) -> (Option<String>, String) {
    let trimmed = name.trim();
    let parts: Vec<&str> = trimmed.splitn(3, |c: char| c.is_whitespace() || c == '/').collect();
    if parts.len() >= 2 {
        let method = parts[0].to_uppercase();
        if matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
        ) {
            let rest = if parts.len() == 2 {
                parts[1].to_string()
            } else {
                format!("{} {}", parts[1], parts[2])
            };
            return (Some(method), rest);
        }
    }
    (None, name.to_string())
}

#[component]
pub fn WorkflowEditor(
    workflow_id: String,
    workflow_name: Signal<String>,
    flow_nodes: Signal<Vec<Node<StudioNodeData>>>,
    flow_edges: Signal<Vec<Edge>>,
    #[props(default = "Projects".to_string())] list_label: String,
    on_back: EventHandler<()>,
    on_save: EventHandler<()>,
    on_run: EventHandler<serde_json::Value>,
    on_add_node: EventHandler<NodeCatalogItem>,
    #[props(default)] on_reload: Option<EventHandler<()>>,
    #[props(default)] refresh_key: u32,
    canvas: Element,
    details: Option<Element>,
    #[props(default)] details_open: bool,
    #[props(default)] run_logs: Signal<Vec<serde_json::Value>>,
    #[props(default)] open_nodes_tick: Signal<u32>,
    #[props(default)] pending_add: Signal<Option<PendingAddRequest>>,
    #[props(default)] workflow_doc: Signal<Option<serde_json::Value>>,
    #[props(default)] doc_nodes: Signal<Vec<serde_json::Value>>,
    #[props(default)] doc_edges: Signal<Vec<serde_json::Value>>,
    #[props(default)] on_toast: Option<EventHandler<String>>,
    #[props(default)] on_open_workflow: Option<EventHandler<String>>,
) -> Element {
    let mut tab = use_signal(|| EditorTab::Editor);
    let mut saving = use_signal(|| false);
    let mut running = use_signal(|| false);
    let mut agent_open = use_signal(|| false);
    let mut nodes_open = use_signal(|| false);
    let mut triggers_only = use_signal(|| false);
    let mut renaming = use_signal(|| false);
    let mut name_draft = use_signal(|| workflow_name());
    let mut menu_open = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let mut delete_busy = use_signal(|| false);
    let mut execute_open = use_signal(|| false);
    let mut execute_busy = use_signal(|| false);
    let mut execute_params = use_signal(|| serde_json::json!({}));
    let mut execute_fields = use_signal(Vec::<FormField>::new);

    let on_execute_click = EventHandler::new(move |_| {
        let (params, fields) = api_endpoint_form_fields(&doc_nodes());
        if let Some(p) = params {
            if needs_execute_dialog(&p, &fields) {
                execute_params.set(p);
                execute_fields.set(fields);
                execute_open.set(true);
            } else {
                // apiEndpoint with no schema / anonymous auth — run immediately.
                on_run.call(build_run_input(&p, &std::collections::HashMap::new()));
            }
        } else {
            on_run.call(serde_json::json!({}));
        }
    });
    let mut history_open = use_signal(|| false);
    let mut history_events = use_signal(Vec::<serde_json::Value>::new);
    let mut history_loading = use_signal(|| false);
    let mut workflow_enabled = use_signal(|| false);
    let mut workflow_version = use_signal(|| "1".to_string());

    let show_details = details_open && details.is_some() && !agent_open();
    let wf_id_meta = workflow_id.clone();
    let wf_id_history = workflow_id.clone();
    let wf_id_runs = workflow_id.clone();
    let wf_id_insights = workflow_id.clone();
    let wf_id_rename_blur = workflow_id.clone();
    let wf_id_rename_key = workflow_id.clone();
    let wf_id_menu_active = workflow_id.clone();
    let wf_id_menu_duplicate = workflow_id.clone();
    let wf_id_menu_download = workflow_id.clone();
    let wf_id_menu_delete = workflow_id.clone();

    use_effect(move || {
        name_draft.set(workflow_name());
    });

    use_effect(move || {
        let tick = open_nodes_tick();
        if tick > 0 {
            triggers_only.set(
                pending_add()
                    .as_ref()
                    .map(|p| p.triggers_only)
                    .unwrap_or(false),
            );
            nodes_open.set(true);
            agent_open.set(false);
        }
    });

    // Kept for App ↔ FlowCanvas overlay wiring.
    let _ = run_logs;

    use_effect(move || {
        let id = wf_id_meta.clone();
        let _ = refresh_key;
        spawn(async move {
            if let Ok(v) = crate::api::get_json(&format!("/api/workflows/{id}")).await {
                let wf = v.get("workflow").cloned().unwrap_or(v);
                workflow_enabled.set(json_bool(&wf, "enabled", false));
                workflow_version.set(wf.get("version").map(json_string).unwrap_or_else(|| "1".into()));
            }
        });
    });

    use_effect(move || {
        if !history_open() {
            return;
        }
        history_loading.set(true);
        let id = wf_id_history.clone();
        let _ = refresh_key;
        spawn(async move {
            let path = format!("/api/audit?resourceId={}&limit=50", urlencoding_quote(&id));
            match crate::api::get_json(&path).await {
                Ok(v) => {
                    history_events.set(
                        v.get("events")
                            .and_then(|e| e.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
                Err(_) => history_events.set(vec![]),
            }
            history_loading.set(false);
        });
    });

    let (name_method, name_rest) = split_http_method_name(&workflow_name());

    rsx! {
        div { class: "relative flex h-full min-w-0 flex-1 flex-col bg-background",
            header { class: "relative flex h-12 shrink-0 items-center gap-3 border-b border-border/70 bg-card/40 px-3 backdrop-blur-md",
                div { class: "flex min-w-0 flex-1 items-center gap-1.5 text-sm",
                    button {
                        r#type: "button",
                        class: "inline-flex items-center gap-1 rounded-md px-1 py-0.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground",
                        onclick: move |_| on_back.call(()),
                        ChevronLeft { class: Some("h-4 w-4".into()) }
                        "{list_label}"
                    }
                    span { class: "text-muted-foreground/70", "/" }
                    if renaming() {
                        input {
                            class: "h-8 max-w-[18rem] rounded-md border bg-background px-1.5 py-0 text-base font-semibold outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            value: "{name_draft()}",
                            oninput: move |e| name_draft.set(e.value()),
                            onblur: {
                                let wf_id = wf_id_rename_blur.clone();
                                move |_| {
                                    let name = name_draft().trim().to_string();
                                    renaming.set(false);
                                    if name.is_empty() || name == workflow_name() {
                                        name_draft.set(workflow_name());
                                    } else {
                                        let id = wf_id.clone();
                                        workflow_name.set(name.clone());
                                        spawn(async move {
                                            let _ = crate::api::put_json(
                                                &format!("/api/workflows/{id}"),
                                                serde_json::json!({ "name": name }),
                                            )
                                            .await;
                                        });
                                    }
                                }
                            },
                            onkeydown: {
                                let wf_id = wf_id_rename_key.clone();
                                move |e| {
                                    if e.key() == Key::Enter {
                                        let name = name_draft().trim().to_string();
                                        renaming.set(false);
                                        if !name.is_empty() && name != workflow_name() {
                                            let id = wf_id.clone();
                                            workflow_name.set(name.clone());
                                            spawn(async move {
                                                let _ = crate::api::put_json(
                                                    &format!("/api/workflows/{id}"),
                                                    serde_json::json!({ "name": name }),
                                                )
                                                .await;
                                            });
                                        } else {
                                            name_draft.set(workflow_name());
                                        }
                                    }
                                    if e.key() == Key::Escape {
                                        name_draft.set(workflow_name());
                                        renaming.set(false);
                                    }
                                }
                            },
                        }
                    } else {
                        button {
                            r#type: "button",
                            title: "Rename",
                            class: "flex min-w-0 items-center gap-1.5 truncate rounded-md px-1 py-0.5 text-base font-semibold tracking-tight hover:bg-muted/50",
                            onclick: move |_| {
                                name_draft.set(workflow_name());
                                renaming.set(true);
                            },
                            if let Some(method) = name_method.clone() {
                                Badge {
                                    variant: BadgeVariant::Default,
                                    class: Some(format!("shrink-0 font-semibold {}", http_method_badge_class(&method))),
                                    "{method}"
                                }
                                span { class: "truncate", "{name_rest}" }
                            } else {
                                span { class: "truncate", "{workflow_name()}" }
                            }
                        }
                    }
                }
                div { class: "relative ml-auto flex flex-1 items-center justify-end gap-2",
                    Button {
                        size: ButtonSize::Sm,
                        class: Some("h-8 shadow-sm".into()),
                        disabled: saving(),
                        onclick: move |_| {
                            saving.set(true);
                            on_save.call(());
                            saving.set(false);
                        },
                        if saving() { "Saving…" } else { "Save" }
                    }
                    Button {
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| history_open.set(true),
                        History { class: Some("h-4 w-4".into()) }
                    }
                    button {
                        r#type: "button",
                        class: "rounded-md p-1.5 text-muted-foreground hover:bg-muted",
                        title: "More",
                        onclick: move |_| menu_open.set(!menu_open()),
                        MoreHorizontal { class: Some("h-4 w-4".into()) }
                    }
                    if menu_open() {
                        div {
                            class: "absolute right-0 top-full z-50 mt-1 min-w-[12rem] rounded-md border bg-card p-1 text-sm shadow-md",
                            onclick: move |e| e.stop_propagation(),
                            div { class: "px-2 py-1.5 text-xs font-medium text-muted-foreground", "Workflow" }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center justify-between gap-4 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: {
                                    move |_| {
                                    let id = wf_id_menu_active.clone();
                                    let next = !workflow_enabled();
                                        workflow_enabled.set(next);
                                        spawn(async move {
                                            let _ = crate::api::put_json(
                                                &format!("/api/workflows/{id}"),
                                                serde_json::json!({ "enabled": next }),
                                            )
                                            .await;
                                        });
                                    }
                                },
                                span {
                                    if workflow_enabled() { "Active" } else { "Inactive" }
                                }
                                Switch {
                                    checked: workflow_enabled(),
                                    size: Some("sm".into()),
                                    on_checked_change: |_| {},
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: move |_| menu_open.set(false),
                                Share2 { class: Some("h-4 w-4".into()) }
                                "Share"
                            }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    history_open.set(true);
                                },
                                History { class: Some("h-4 w-4".into()) }
                                "History"
                            }
                            div { class: "my-1 h-px bg-border" }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    name_draft.set(workflow_name());
                                    renaming.set(true);
                                },
                                Pencil { class: Some("h-4 w-4".into()) }
                                "Rename"
                            }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: {
                                    move |_| {
                                        menu_open.set(false);
                                        let id = wf_id_menu_duplicate.clone();
                                        let toast_cb = on_toast;
                                        let open_cb = on_open_workflow;
                                        let reload_cb = on_reload;
                                        let name = workflow_name();
                                        let (nodes, edges) = merge_flow_into_workflow_json(
                                            doc_nodes(),
                                            doc_edges(),
                                            &flow_nodes(),
                                            &flow_edges(),
                                        );
                                        doc_nodes.set(nodes.clone());
                                        doc_edges.set(edges.clone());
                                        let mut save_body = workflow_doc()
                                            .unwrap_or_else(|| serde_json::json!({}));
                                        if let Some(obj) = save_body.as_object_mut() {
                                            obj.insert(
                                                "name".into(),
                                                serde_json::Value::String(name),
                                            );
                                            obj.insert(
                                                "nodes".into(),
                                                serde_json::Value::Array(nodes),
                                            );
                                            obj.insert(
                                                "edges".into(),
                                                serde_json::Value::Array(edges),
                                            );
                                            obj.remove("_id");
                                        }
                                        spawn(async move {
                                            if let Err(e) = crate::api::put_json(
                                                &format!("/api/workflows/{id}"),
                                                save_body,
                                            )
                                            .await
                                            {
                                                if let Some(cb) = toast_cb {
                                                    cb.call(format!("Duplicate failed: {e}"));
                                                }
                                                return;
                                            }
                                            let latest = match crate::api::get_json(
                                                &format!("/api/workflows/{id}"),
                                            )
                                            .await
                                            {
                                                Ok(v) => v.get("workflow").cloned().unwrap_or(v),
                                                Err(e) => {
                                                    if let Some(cb) = toast_cb {
                                                        cb.call(format!("Duplicate failed: {e}"));
                                                    }
                                                    return;
                                                }
                                            };
                                            let wf_name = latest
                                                .get("name")
                                                .map(json_string)
                                                .unwrap_or_default();
                                            let body = serde_json::json!({
                                                "name": format!("{wf_name} copy"),
                                                "description": latest.get("description").cloned(),
                                                "enabled": false,
                                                "nodes": latest.get("nodes").cloned().unwrap_or_default(),
                                                "edges": latest.get("edges").cloned().unwrap_or_default(),
                                                "tags": latest.get("tags").cloned().unwrap_or(serde_json::json!([])),
                                                "timeSavedMinutes": latest.get("timeSavedMinutes").cloned(),
                                            });
                                            match crate::api::post_json("/api/workflows", body).await {
                                                Ok(v) => {
                                                    let new_id = v
                                                        .get("workflow")
                                                        .map(json_id)
                                                        .unwrap_or_else(|| json_id(&v));
                                                    if let Some(cb) = toast_cb {
                                                        cb.call("Workflow duplicated".into());
                                                    }
                                                    if let Some(cb) = reload_cb {
                                                        cb.call(());
                                                    }
                                                    if !new_id.is_empty() {
                                                        if let Some(cb) = open_cb {
                                                            cb.call(new_id);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    if let Some(cb) = toast_cb {
                                                        cb.call(format!("Duplicate failed: {e}"));
                                                    }
                                                }
                                            }
                                        });
                                    }
                                },
                                Copy { class: Some("h-4 w-4".into()) }
                                "Duplicate"
                            }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    let name = workflow_name();
                                    let (nodes, edges) = merge_flow_into_workflow_json(
                                        doc_nodes(),
                                        doc_edges(),
                                        &flow_nodes(),
                                        &flow_edges(),
                                    );
                                    let doc = workflow_doc();
                                    let payload = serde_json::json!({
                                        "name": name,
                                        "description": doc.as_ref().and_then(|d| d.get("description").cloned()),
                                        "enabled": workflow_enabled(),
                                        "version": workflow_version(),
                                        "tags": doc.as_ref()
                                            .and_then(|d| d.get("tags").cloned())
                                            .unwrap_or_else(|| serde_json::json!([])),
                                        "timeSavedMinutes": doc.as_ref().and_then(|d| d.get("timeSavedMinutes").cloned()),
                                        "nodes": nodes,
                                        "edges": edges,
                                    });
                                    let Ok(json) = serde_json::to_string_pretty(&payload) else {
                                        if let Some(cb) = on_toast {
                                            cb.call("Download failed".into());
                                        }
                                        return;
                                    };
                                    let filename = format!("{}.json", sanitize_workflow_filename(&name));
                                    let toast_cb = on_toast;
                                    let _ = wf_id_menu_download;
                                    // Browser-style Blob download (matches web WorkflowEditor).
                                    let json_js = serde_json::to_string(&json)
                                        .unwrap_or_else(|_| "\"{}\"".into());
                                    let filename_js = serde_json::to_string(&filename)
                                        .unwrap_or_else(|_| "\"workflow.json\"".into());
                                    let _ = document::eval(&format!(
                                        r#"(function(){{
  const blob = new Blob([{json_js}], {{ type: 'application/json' }});
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = {filename_js};
  a.click();
  URL.revokeObjectURL(url);
}})()"#,
                                    ));
                                    if let Some(cb) = toast_cb {
                                        cb.call("Workflow downloaded".into());
                                    }
                                },
                                Download { class: Some("h-4 w-4".into()) }
                                "Download"
                            }
                            div { class: "my-1 h-px bg-border" }
                            button {
                                r#type: "button",
                                class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-destructive hover:bg-destructive/10",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    confirm_delete.set(true);
                                },
                                Trash2 { class: Some("h-4 w-4".into()) }
                                "Delete"
                            }
                        }
                    }
                }
            }

            if history_open() {
                div {
                    class: "fixed inset-y-0 right-0 z-[150] flex w-[min(100vw,22rem)] flex-col border-l bg-card shadow-xl",
                    div { class: "border-b px-4 py-4",
                        div { class: "flex items-start justify-between gap-2",
                            div {
                                h2 { class: "text-base font-semibold", "History" }
                                p { class: "text-xs text-muted-foreground",
                                    "Audit trail for this workflow (v{workflow_version()})."
                                }
                            }
                            button {
                                class: "inline-flex h-7 w-7 items-center justify-center rounded-md hover:bg-muted",
                                onclick: move |_| history_open.set(false),
                                X { class: Some("h-4 w-4".into()) }
                            }
                        }
                    }
                    div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-4",
                        div { class: "flex flex-col gap-2 pt-2",
                            for ev in history_events().iter().cloned() {
                                {
                                    let action = ev.get("action").map(json_string).unwrap_or_default();
                                    let actor = ev.get("actor").map(json_string).unwrap_or_default();
                                    let at = ev.get("at").map(json_string).unwrap_or_default();
                                    rsx! {
                                        div {
                                            key: "{json_id(&ev)}",
                                            class: "rounded-md border bg-card p-2 text-sm",
                                            Badge { variant: BadgeVariant::Outline, "{action}" }
                                            div { class: "mt-1 text-xs text-muted-foreground",
                                                span { class: "font-medium text-foreground", "{actor}" }
                                                " · {at}"
                                            }
                                        }
                                    }
                                }
                            }
                            if history_events().is_empty() && !history_loading() {
                                div { class: "rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground",
                                    "No history yet. Save or run this workflow to populate it."
                                }
                            }
                            if history_loading() {
                                div { class: "text-sm text-muted-foreground", "Loading history…" }
                            }
                        }
                    }
                }
                div {
                    class: "fixed inset-0 z-[140] bg-black/20",
                    onclick: move |_| history_open.set(false),
                }
            }

            div { class: "pointer-events-none absolute left-1/2 top-12 z-[100] flex h-8 -translate-x-1/2 -translate-y-1/2 items-stretch rounded-md bg-muted p-0.5 shadow-sm",
                button {
                    class: if tab() == EditorTab::Editor {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm bg-background px-3.5 text-xs font-medium leading-none text-foreground"
                    } else {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm px-3.5 text-xs font-medium leading-none text-muted-foreground hover:bg-background/50 hover:text-foreground"
                    },
                    onclick: move |_| tab.set(EditorTab::Editor),
                    "Editor"
                }
                button {
                    class: if tab() == EditorTab::Executions {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm bg-background px-3.5 text-xs font-medium leading-none text-foreground"
                    } else {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm px-3.5 text-xs font-medium leading-none text-muted-foreground hover:bg-background/50 hover:text-foreground"
                    },
                    onclick: move |_| tab.set(EditorTab::Executions),
                    "Executions"
                }
                button {
                    class: if tab() == EditorTab::Insights {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm bg-background px-3.5 text-xs font-medium leading-none text-foreground"
                    } else {
                        "pointer-events-auto inline-flex items-center justify-center rounded-sm px-3.5 text-xs font-medium leading-none text-muted-foreground hover:bg-background/50 hover:text-foreground"
                    },
                    onclick: move |_| tab.set(EditorTab::Insights),
                    "Insights"
                }
            }

            // flex + flex-col so editor_main's flex-1/h-full resolve to a
            // real height — dioxus-flow paints nothing when container is 0×0.
            // Executions and Insights replace the canvas (full workspace).
            div { class: "relative flex min-h-0 flex-1 flex-col",
                if tab() == EditorTab::Executions {
                    div { class: "min-h-0 flex-1 overflow-hidden",
                        RunsPanel {
                            refresh_key,
                            workflow_id: Some(wf_id_runs.clone()),
                        }
                    }
                } else if tab() == EditorTab::Insights {
                    div { class: "min-h-0 flex-1 overflow-hidden",
                        WorkflowApiInsights {
                            workflow_id: wf_id_insights.clone(),
                            refresh_key,
                        }
                    }
                } else {
                    {editor_main(
                        agent_open,
                        nodes_open,
                        triggers_only,
                        show_details,
                        running,
                        on_add_node,
                        on_execute_click,
                        on_reload,
                        canvas,
                        details,
                    )}
                }
            }

            div { class: "sr-only",
                "Workflow {workflow_id} — nodes: {flow_nodes().len()}, edges: {flow_edges().len()}"
            }

            if execute_open() {
                ExecuteWorkflowDialog {
                    parameters: execute_params(),
                    fields: execute_fields(),
                    busy: execute_busy(),
                    on_cancel: move |_| {
                        if !execute_busy() {
                            execute_open.set(false);
                        }
                    },
                    on_execute: move |input: serde_json::Value| {
                        execute_busy.set(true);
                        running.set(true);
                        on_run.call(input);
                        execute_busy.set(false);
                        running.set(false);
                        execute_open.set(false);
                    },
                }
            }

            if confirm_delete() {
                {
                    let id = wf_id_menu_delete.clone();
                    let name = workflow_name();
                    let description = format!(
                        "This will permanently delete workflow \"{name}\". This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete workflow?".to_string(),
                            description,
                            busy: delete_busy(),
                            on_cancel: move |_| confirm_delete.set(false),
                            on_confirm: move |_| {
                                let id = id.clone();
                                delete_busy.set(true);
                                spawn(async move {
                                    if crate::api::delete_json(&format!("/api/workflows/{id}")).await.is_ok() {
                                        confirm_delete.set(false);
                                        on_back.call(());
                                    }
                                    delete_busy.set(false);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}

fn sanitize_workflow_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "workflow".into()
    } else {
        sanitized
    }
}

fn urlencoding_quote(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn editor_main(
    agent_open: Signal<bool>,
    nodes_open: Signal<bool>,
    triggers_only: Signal<bool>,
    show_details: bool,
    running: Signal<bool>,
    on_add_node: EventHandler<NodeCatalogItem>,
    on_execute_click: EventHandler<()>,
    on_reload: Option<EventHandler<()>>,
    canvas: Element,
    details: Option<Element>,
) -> Element {
    rsx! {
        div { class: "relative flex h-full min-h-0 w-full flex-1",
            div { class: "relative h-full min-h-0 min-w-0 flex-1",
                {editor_canvas_stack(
                    agent_open,
                    nodes_open,
                    triggers_only,
                    running,
                    on_add_node,
                    on_execute_click,
                    on_reload,
                    canvas,
                )}
                if show_details {
                    if let Some(details_el) = details {
                        div { class: "absolute inset-0 z-40 min-w-0",
                            {details_el}
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn editor_canvas_stack(
    mut agent_open: Signal<bool>,
    mut nodes_open: Signal<bool>,
    mut triggers_only: Signal<bool>,
    running: Signal<bool>,
    on_add_node: EventHandler<NodeCatalogItem>,
    on_execute_click: EventHandler<()>,
    on_reload: Option<EventHandler<()>>,
    canvas: Element,
) -> Element {
    rsx! {
        div { class: "relative h-full min-h-0 min-w-0 w-full",
            // Fill the editor pane explicitly — percentage height alone collapses
            // when an ancestor is not a flex/sized box, leaving a blank canvas.
            div { class: "absolute inset-0 min-h-0 min-w-0",
                {canvas}
            }

            div { class: "absolute right-3 top-3 z-10 flex items-center gap-2",
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Outline,
                    class: Some("h-8 border-border/80 bg-card/95 shadow-md backdrop-blur-sm".into()),
                    onclick: move |_| {
                        let next = !agent_open();
                        agent_open.set(next);
                        if next { nodes_open.set(false); }
                    },
                    MessageSquare { class: Some("h-4 w-4".into()) }
                    "Agent"
                }
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Outline,
                    class: Some("h-8 border-border/80 bg-card/95 shadow-md backdrop-blur-sm".into()),
                    onclick: move |_| {
                        triggers_only.set(false);
                        nodes_open.set(true);
                        agent_open.set(false);
                    },
                    Boxes { class: Some("h-4 w-4".into()) }
                    "Nodes"
                }
            }

            if agent_open() {
                aside { class: "absolute inset-y-0 right-0 z-20 flex w-[min(100%,36rem)] flex-col border-l bg-card shadow-xl",
                    ChatPanel {
                        compact: true,
                        on_workflow_changed: on_reload,
                    }
                }
            }

            if nodes_open() {
                div {
                    class: "absolute inset-0 z-50 flex items-start justify-center bg-black/40 p-4 pt-[8vh] backdrop-blur-[1px]",
                    onclick: move |_| {
                        nodes_open.set(false);
                        triggers_only.set(false);
                    },
                    div {
                        class: "flex max-h-[80vh] w-full max-w-3xl flex-col overflow-hidden rounded-xl border bg-card shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        role: "dialog",
                        "aria-modal": "true",
                        "aria-label": if triggers_only() { "Choose a trigger" } else { "Add node" },
                        div { class: "flex items-start justify-between gap-3 border-b px-4 py-3",
                            div {
                                h2 { class: "text-base font-semibold",
                                    if triggers_only() {
                                        "Choose a trigger"
                                    } else {
                                        "What happens next?"
                                    }
                                }
                                p { class: "text-xs text-muted-foreground",
                                    if triggers_only() {
                                        "Pick how this workflow should start."
                                    } else {
                                        "Search or pick a node. Tab opens this panel · Esc closes."
                                    }
                                }
                            }
                            button {
                                class: "inline-flex h-7 w-7 items-center justify-center rounded-md hover:bg-muted",
                                "aria-label": "Close",
                                onclick: move |_| {
                                    nodes_open.set(false);
                                    triggers_only.set(false);
                                },
                                X { class: Some("h-4 w-4".into()) }
                            }
                        }
                        div { class: "min-h-0 flex-1 overflow-hidden",
                            NodesPanel {
                                compact: true,
                                hide_header: true,
                                can_add: true,
                                variant: NodesPanelVariant::Grid,
                                triggers_only: triggers_only(),
                                on_add_node: move |item: NodeCatalogItem| {
                                    on_add_node.call(item);
                                    nodes_open.set(false);
                                    triggers_only.set(false);
                                },
                            }
                        }
                    }
                }
            }

            div { class: "pointer-events-none absolute inset-x-0 bottom-4 z-10 flex justify-center",
                Button {
                    size: ButtonSize::Lg,
                    class: Some("pointer-events-auto h-11 rounded-xl px-6 text-[13px] font-semibold tracking-tight shadow-[0_8px_28px_color-mix(in_oklch,var(--brand)_35%,transparent)]".into()),
                    disabled: running(),
                    onclick: move |_| on_execute_click.call(()),
                    FlaskConical { class: Some("h-4 w-4".into()) }
                    if running() { "Executing…" } else { "Execute workflow" }
                }
            }
        }
    }
}