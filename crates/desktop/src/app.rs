use dioxus::prelude::*;
use dioxus_flow::prelude::{Edge, Node};

use crate::components::flow::{
    merge_flow_into_workflow_json, new_catalog_node, workflow_json_to_flow, FlowCanvas,
    PendingAddRequest, StudioNodeData,
};
use crate::components::layout::{MainSidebar, NavSection, WorkflowEditor};
use crate::components::node_schemas::{get_schema, sanitize_parameters};
use crate::components::panels::{
    sample_from_api_endpoint_parameters, ApiProjectsPanel, AuditPanel, CredentialsPanel,
    DashboardPanel, InsightsPanel, LoginPage, NodeCatalogItem, NodeDetailsPanel, NodeUpdate,
    NodesPanel, RunsPanel, VariablesPanel, WorkflowList,
};
use crate::components::settings::{SettingsPageId, SettingsPanel, SettingsSidebar};
use crate::components::icons::{CircleAlert, CircleCheck};
use crate::components::ui::{Alert, AlertDescription, AlertTitle, AlertVariant, PageLoader};
use crate::theme::Theme;

fn toast_variant(msg: &str) -> AlertVariant {
    let lower = msg.to_ascii_lowercase();
    if lower.contains("fail")
        || lower.contains("error")
        || lower.contains("denied")
        || lower.contains("invalid")
        || lower.contains("unable")
        || lower.contains("not found")
    {
        AlertVariant::Destructive
    } else if lower.contains("complet")
        || lower.contains("saved")
        || lower.contains("created")
        || lower.contains("added")
        || lower.contains("started")
        || lower.contains("executed")
    {
        AlertVariant::Success
    } else {
        AlertVariant::Default
    }
}

fn toast_title(variant: AlertVariant) -> &'static str {
    match variant {
        AlertVariant::Destructive => "Something went wrong",
        AlertVariant::Success => "Success",
        AlertVariant::Default => "Notice",
    }
}

#[derive(Clone)]
struct SessionUser {
    name: String,
    role: String,
}

#[component]
pub fn App() -> Element {
    crate::codemirror_assets::use_asset_routes();
    crate::codemirror_assets::preload();

    let mut theme = use_signal(|| Theme::Dark);
    let mut auth_loading = use_signal(|| true);
    let mut session = use_signal(|| None::<SessionUser>);
    let mut section = use_signal(|| NavSection::Dashboard);
    let mut settings_page = use_signal(|| SettingsPageId::Profile);
    let mut collapsed = use_signal(|| false);
    let mut refresh_key = use_signal(|| 0u32);
    let mut editor_open = use_signal(|| false);
    let mut selected_id = use_signal(|| None::<String>);
    let mut api_project_id = use_signal(|| None::<String>);
    let mut workflow_name = use_signal(|| "Workflow".to_string());
    let mut workflow_doc = use_signal(|| None::<serde_json::Value>);
    let mut doc_nodes = use_signal(Vec::<serde_json::Value>::new);
    let mut doc_edges = use_signal(Vec::<serde_json::Value>::new);
    let mut flow_nodes = use_signal(Vec::<Node<StudioNodeData>>::new);
    let mut flow_edges = use_signal(Vec::<Edge>::new);
    let mut creating = use_signal(|| false);
    let mut workflow_loading = use_signal(|| false);
    let mut toast = use_signal(|| None::<String>);
    let mut details_open = use_signal(|| false);
    let mut run_logs = use_signal(Vec::<serde_json::Value>::new);
    let mut pending_add = use_signal(|| None::<PendingAddRequest>);
    let mut open_nodes_tick = use_signal(|| 0u32);
    let mut node_output_samples = use_signal(std::collections::HashMap::<String, serde_json::Value>::new);

    let auth_disabled = std::env::var("BOTFLOW_AUTH_DISABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    use_effect(move || {
        spawn(async move {
            if auth_disabled {
                session.set(Some(SessionUser {
                    name: "local".into(),
                    role: "admin".into(),
                }));
            } else if let Ok(me) = crate::api::get_json("/api/me").await {
                let name = me
                    .get("username")
                    .or_else(|| me.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("user")
                    .to_string();
                let role = me
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user")
                    .to_string();
                session.set(Some(SessionUser { name, role }));
            }
            auth_loading.set(false);
        });
    });

    let mut reload_workflow = move |id: String| {
        workflow_loading.set(true);
        // Show the canvas when opening a workflow (e.g. API method row) —
        // do not auto-open the first node's editor.
        details_open.set(false);
        workflow_doc.set(None);
        flow_nodes.set(vec![]);
        flow_edges.set(vec![]);
        doc_nodes.set(vec![]);
        doc_edges.set(vec![]);
        run_logs.set(vec![]);
        workflow_name.set("Loading…".into());
        spawn(async move {
            match crate::api::get_json(&format!("/api/workflows/{id}")).await {
                Ok(v) => {
                    let wf = v.get("workflow").cloned().unwrap_or(v);
                    let name = wf
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("Workflow")
                        .to_string();
                    workflow_name.set(name);
                    let nodes = wf
                        .get("nodes")
                        .and_then(|n| n.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let edges = wf
                        .get("edges")
                        .and_then(|n| n.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let (fnodes, fedges) = workflow_json_to_flow(&nodes, &edges);
                    doc_nodes.set(nodes);
                    doc_edges.set(edges);
                    flow_nodes.set(fnodes);
                    flow_edges.set(fedges);
                    workflow_doc.set(Some(wf));
                    workflow_loading.set(false);
                }
                Err(e) => {
                    workflow_loading.set(false);
                    toast.set(Some(e));
                }
            }
        });
    };

    let theme_class = theme().as_class();
    let root_class = format!("flex h-screen w-screen overflow-hidden bg-background text-foreground {theme_class}");

    use_effect(move || {
        let class = theme().as_class();
        document::eval(&format!(
            r#"document.documentElement.classList.toggle('dark', {0});
               document.body.classList.toggle('dark', {0});
               document.documentElement.style.height='100%';
               document.body.style.height='100%';
               document.body.style.margin='0';"#,
            class == "dark"
        ));
    });

    use_effect(move || {
        let Some(msg) = toast() else { return };
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            if toast.peek().as_ref() == Some(&msg) {
                toast.set(None);
            }
        });
    });

    if auth_loading() {
        return rsx! {
            style { dangerous_inner_html: crate::styles::STUDIO_CSS }
            div { class: "{root_class}",
                PageLoader { label: "Loading…" }
            }
        };
    }

    if session().is_none() {
        return rsx! {
            style { dangerous_inner_html: crate::styles::STUDIO_CSS }
            div { class: "{root_class}",
                LoginPage {
                    on_logged_in: move |(name, role)| {
                        session.set(Some(SessionUser { name, role }));
                    }
                }
            }
        };
    }

    let user = session().unwrap();
    let actor_name = user.name.clone();
    let role = user.role.clone();
    let in_settings = section() == NavSection::Settings;
    let show_editor = (section() == NavSection::Workflows || section() == NavSection::Apis)
        && editor_open()
        && selected_id().is_some();

    rsx! {
        style { dangerous_inner_html: crate::styles::STUDIO_CSS }
        div {
            class: "{root_class}",
            if in_settings {
                SettingsSidebar {
                    page: settings_page(),
                    is_admin: role == "admin",
                    on_select: move |p| settings_page.set(p),
                    on_back: move |_| {
                        section.set(NavSection::Dashboard);
                        editor_open.set(false);
                    },
                }
            } else {
                MainSidebar {
                    section: section(),
                    collapsed: collapsed(),
                    theme: theme(),
                    actor_name: actor_name.clone(),
                    role: role.clone(),
                    on_navigate: move |s| {
                        section.set(s);
                        if s == NavSection::Apis {
                            // Sidebar → list; keep selection only when Back from editor.
                            api_project_id.set(None);
                            editor_open.set(false);
                        } else if s != NavSection::Workflows {
                            editor_open.set(false);
                        }
                    },
                    on_new_workflow: move |_| {
                        creating.set(true);
                        spawn(async move {
                            let body = serde_json::json!({ "name": "My workflow" });
                            match crate::api::post_json("/api/workflows", body).await {
                                Ok(v) => {
                                    let id = v
                                        .get("workflow")
                                        .and_then(|w| w.get("_id").or_else(|| w.get("id")))
                                        .and_then(|x| x.as_str())
                                        .map(String::from);
                                    refresh_key.with_mut(|k| *k += 1);
                                    if let Some(id) = id {
                                        selected_id.set(Some(id.clone()));
                                        section.set(NavSection::Workflows);
                                        editor_open.set(true);
                                        reload_workflow(id);
                                    }
                                    toast.set(Some("Workflow created".into()));
                                }
                                Err(e) => toast.set(Some(e)),
                            }
                            creating.set(false);
                        });
                    },
                    on_toggle_collapse: move |_| collapsed.with_mut(|v| *v = !*v),
                    on_toggle_theme: move |_| theme.with_mut(|t| *t = t.toggle()),
                    on_profile: move |_| {
                        settings_page.set(SettingsPageId::Profile);
                        section.set(NavSection::Settings);
                        editor_open.set(false);
                    },
                    on_open_settings: move |page: SettingsPageId| {
                        settings_page.set(page);
                        section.set(NavSection::Settings);
                        editor_open.set(false);
                    },
                    on_logout: move |_| {
                        crate::api::set_auth_token(None);
                        session.set(None);
                        section.set(NavSection::Dashboard);
                        editor_open.set(false);
                    },
                }
            }

            div { class: "flex min-h-0 min-w-0 flex-1 flex-col",
                if show_editor {
                    if workflow_loading() || workflow_doc().is_none() {
                        PageLoader { label: "Loading workflow…" }
                    } else if let Some(id) = selected_id() {
                        WorkflowEditor {
                            workflow_id: id.clone(),
                            workflow_name,
                            flow_nodes,
                            flow_edges,
                            list_label: if section() == NavSection::Apis {
                                "API Projects".to_string()
                            } else {
                                "Projects".to_string()
                            },
                            refresh_key: refresh_key(),
                            on_back: move |_| {
                                editor_open.set(false);
                                if section() == NavSection::Apis {
                                    // Keep api_project_id so detail view restores.
                                }
                            },
                            on_reload: Some(EventHandler::new(move |_| {
                                if let Some(id) = selected_id() {
                                    reload_workflow(id);
                                }
                            })),
                            on_save: move |_| {
                                let id = selected_id().unwrap_or_default();
                                let name = workflow_name();
                                let (nodes, edges) = merge_flow_into_workflow_json(
                                    doc_nodes(),
                                    doc_edges(),
                                    &flow_nodes(),
                                    &flow_edges(),
                                );
                                doc_nodes.set(nodes.clone());
                                doc_edges.set(edges.clone());
                                let mut body = workflow_doc().unwrap_or_else(|| serde_json::json!({}));
                                if let Some(obj) = body.as_object_mut() {
                                    obj.insert("name".into(), serde_json::Value::String(name));
                                    obj.insert(
                                        "nodes".into(),
                                        serde_json::Value::Array(nodes),
                                    );
                                    obj.insert(
                                        "edges".into(),
                                        serde_json::Value::Array(edges),
                                    );
                                    // Server expects WorkflowDoc; keep id out of replace payload noise.
                                    obj.remove("_id");
                                }
                                spawn(async move {
                                    match crate::api::put_json(
                                        &format!("/api/workflows/{id}"),
                                        body,
                                    )
                                    .await
                                    {
                                        Ok(v) => {
                                            if let Some(wf) =
                                                v.get("workflow").cloned().or(Some(v))
                                            {
                                                workflow_doc.set(Some(wf));
                                            }
                                            toast.set(Some("Saved".into()));
                                        }
                                        Err(e) => toast.set(Some(e)),
                                    }
                                });
                            },
                            on_run: move |input: serde_json::Value| {
                                let id = selected_id().unwrap_or_default();
                                // Persist graph before run so executions match the canvas.
                                let name = workflow_name();
                                let (nodes, edges) = merge_flow_into_workflow_json(
                                    doc_nodes(),
                                    doc_edges(),
                                    &flow_nodes(),
                                    &flow_edges(),
                                );
                                doc_nodes.set(nodes.clone());
                                doc_edges.set(edges.clone());
                                let mut body = workflow_doc().unwrap_or_else(|| serde_json::json!({}));
                                if let Some(obj) = body.as_object_mut() {
                                    obj.insert("name".into(), serde_json::Value::String(name));
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
                                    let _ = crate::api::put_json(
                                        &format!("/api/workflows/{id}"),
                                        body,
                                    )
                                    .await;
                                    match crate::api::post_json(
                                        &format!("/api/workflows/{id}/run"),
                                        serde_json::json!({ "input": input }),
                                    )
                                    .await
                                    {
                                        Ok(started) => {
                                            refresh_key.with_mut(|k| *k += 1);
                                            toast.set(Some("Run started".into()));
                                            // /run returns immediately (queued). Poll until the
                                            // scheduler finishes so canvas overlays don't stick on "…".
                                            let run_id = started
                                                .get("run")
                                                .and_then(|r| r.get("_id").or_else(|| r.get("id")))
                                                .and_then(|v| v.as_str())
                                                .map(str::to_string);
                                            if let Some(logs) = started
                                                .get("run")
                                                .and_then(|r| r.get("logs"))
                                                .and_then(|l| l.as_array())
                                                .cloned()
                                            {
                                                run_logs.set(logs);
                                            }
                                            if let Some(run_id) = run_id {
                                                for _ in 0..60 {
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_millis(250),
                                                    )
                                                    .await;
                                                    let Ok(detail) = crate::api::get_json(
                                                        &format!("/api/runs/{run_id}"),
                                                    )
                                                    .await
                                                    else {
                                                        continue;
                                                    };
                                                    let run = detail.get("run").unwrap_or(&detail);
                                                    if let Some(logs) = run
                                                        .get("logs")
                                                        .and_then(|l| l.as_array())
                                                        .cloned()
                                                    {
                                                        run_logs.set(logs);
                                                    }
                                                    let status = run
                                                        .get("status")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or("");
                                                    if matches!(
                                                        status,
                                                        "completed"
                                                            | "failed"
                                                            | "cancelled"
                                                            | "canceled"
                                                    ) {
                                                        refresh_key.with_mut(|k| *k += 1);
                                                        if status == "failed" {
                                                            let err = run
                                                                .get("error")
                                                                .and_then(|e| e.as_str())
                                                                .unwrap_or("Run failed");
                                                            toast.set(Some(err.to_string()));
                                                        } else if status == "completed" {
                                                            toast.set(Some("Run completed".into()));
                                                        }
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => toast.set(Some(e)),
                                    }
                                });
                            },
                            on_add_node: move |item: NodeCatalogItem| {
                                let idx = doc_nodes().len();
                                let pending = pending_add();
                                let connect_from = pending
                                    .as_ref()
                                    .and_then(|p| p.connect_from.clone());
                                let source_handle = pending
                                    .as_ref()
                                    .and_then(|p| p.source_handle.clone());
                                let source_pos = connect_from.as_ref().and_then(|sid| {
                                    flow_nodes().iter().find(|n| n.id == *sid).map(|n| {
                                        (n.position.x, n.position.y)
                                    })
                                });
                                let (raw, flow, edge) = new_catalog_node(
                                    &item.node_type,
                                    &item.label,
                                    idx,
                                    connect_from.as_deref(),
                                    source_handle.as_deref(),
                                    source_pos,
                                );
                                doc_nodes.with_mut(|n| n.push(raw));
                                flow_nodes.with_mut(|n| {
                                    for node in n.iter_mut() {
                                        node.selected = false;
                                    }
                                    let mut flow = flow;
                                    flow.selected = true;
                                    n.push(flow);
                                });
                                if let Some(e) = edge {
                                    flow_edges.with_mut(|es| es.push(e));
                                }
                                pending_add.set(None);
                                details_open.set(true);
                                toast.set(Some(format!("Added {}", item.label)));
                                // Persist immediately so a later server sync cannot wipe the new node.
                                let id = selected_id().unwrap_or_default();
                                let name = workflow_name();
                                let (nodes, edges) = merge_flow_into_workflow_json(
                                    doc_nodes(),
                                    doc_edges(),
                                    &flow_nodes(),
                                    &flow_edges(),
                                );
                                doc_nodes.set(nodes.clone());
                                doc_edges.set(edges.clone());
                                let mut body = workflow_doc().unwrap_or_else(|| serde_json::json!({}));
                                if let Some(obj) = body.as_object_mut() {
                                    obj.insert("name".into(), serde_json::Value::String(name));
                                    obj.insert("nodes".into(), serde_json::Value::Array(nodes));
                                    obj.insert("edges".into(), serde_json::Value::Array(edges));
                                    obj.remove("_id");
                                }
                                spawn(async move {
                                    let _ = crate::api::put_json(
                                        &format!("/api/workflows/{id}"),
                                        body,
                                    )
                                    .await;
                                });
                            },
                            workflow_doc,
                            doc_nodes,
                            doc_edges,
                            on_toast: move |msg: String| toast.set(Some(msg)),
                            on_open_workflow: move |new_id: String| {
                                selected_id.set(Some(new_id.clone()));
                                editor_open.set(true);
                                reload_workflow(new_id);
                                refresh_key.with_mut(|k| *k += 1);
                            },
                            run_logs,
                            open_nodes_tick,
                            pending_add,
                            canvas: rsx! {
                                FlowCanvas {
                                    nodes: flow_nodes,
                                    edges: flow_edges,
                                    doc_nodes,
                                    run_logs,
                                    fit_view: true,
                                    on_node_select: move |id: Option<String>| {
                                        if let Some(id) = id {
                                            flow_nodes.with_mut(|nodes| {
                                                for n in nodes.iter_mut() {
                                                    n.selected = n.id == id;
                                                }
                                            });
                                            details_open.set(true);
                                        } else {
                                            flow_nodes.with_mut(|nodes| {
                                                for n in nodes.iter_mut() {
                                                    n.selected = false;
                                                }
                                            });
                                        }
                                    },
                                    on_request_add: move |req: PendingAddRequest| {
                                        pending_add.set(Some(req));
                                        open_nodes_tick.with_mut(|t| *t += 1);
                                    },
                                    on_execute_node: move |node_id: String| {
                                        let wid = selected_id().unwrap_or_default();
                                        let doc = doc_nodes().into_iter().find(|n| {
                                            n.get("id").and_then(|v| v.as_str())
                                                == Some(node_id.as_str())
                                        });
                                        let params = doc
                                            .as_ref()
                                            .and_then(|n| n.get("parameters").cloned());
                                        let node_type = doc
                                            .and_then(|n| n.get("type").and_then(|v| v.as_str()).map(String::from));
                                        let sanitized = match (&node_type, &params) {
                                            (Some(ty), Some(p)) => {
                                                if let Some(schema) = get_schema(ty) {
                                                    match sanitize_parameters(schema, p) {
                                                        Ok(clean) => Some(clean),
                                                        Err(e) => {
                                                            toast.set(Some(e));
                                                            return;
                                                        }
                                                    }
                                                } else {
                                                    params.clone()
                                                }
                                            }
                                            _ => params,
                                        };
                                        if let Some(clean) = sanitized.clone() {
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str())
                                                        == Some(node_id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        obj.insert("parameters".into(), clean);
                                                    }
                                                }
                                            });
                                        }
                                        let step_input = if node_type.as_deref() == Some("apiEndpoint") {
                                            sanitized
                                                .as_ref()
                                                .and_then(sample_from_api_endpoint_parameters)
                                        } else {
                                            None
                                        };
                                        spawn(async move {
                                            let mut body = serde_json::json!({
                                                "nodeId": node_id,
                                                "parameters": sanitized,
                                            });
                                            if let Some(input) = step_input {
                                                if let Some(obj) = body.as_object_mut() {
                                                    obj.insert("input".into(), input);
                                                }
                                            }
                                            match crate::api::post_json(
                                                &format!("/api/workflows/{wid}/execute-node"),
                                                body,
                                            )
                                            .await
                                            {
                                                Ok(v) => {
                                                    if let Some(err) =
                                                        v.get("error").and_then(|e| e.as_str())
                                                    {
                                                        toast.set(Some(err.to_string()));
                                                    } else {
                                                        if let Some(out) = v.get("output") {
                                                            node_output_samples.with_mut(|m| {
                                                                m.insert(node_id.clone(), out.clone());
                                                            });
                                                        }
                                                        toast.set(Some("Step executed".into()));
                                                        details_open.set(true);
                                                    }
                                                }
                                                Err(e) => toast.set(Some(e)),
                                            }
                                        });
                                    },
                                    on_toast: move |msg: String| toast.set(Some(msg)),
                                }
                            },
                            details: Some(rsx! {
                                NodeDetailsPanel {
                                    flow_nodes,
                                    doc_nodes,
                                    doc_edges,
                                    run_logs,
                                    node_samples: node_output_samples,
                                    workflow_id: selected_id(),
                                    workflow_enabled: workflow_doc()
                                        .as_ref()
                                        .map(|wf| crate::components::panels::json_bool(wf, "enabled", false))
                                        .unwrap_or(false),
                                    on_toast: move |msg: String| toast.set(Some(msg)),
                                    on_pin_sample: move |(node_id, sample): (String, serde_json::Value)| {
                                        node_output_samples.with_mut(|m| {
                                            m.insert(node_id, sample);
                                        });
                                    },
                                    on_execute_node: move |node_id: String| {
                                        let wid = selected_id().unwrap_or_default();
                                        let doc = doc_nodes().into_iter().find(|n| {
                                            n.get("id").and_then(|v| v.as_str())
                                                == Some(node_id.as_str())
                                        });
                                        let params = doc
                                            .as_ref()
                                            .and_then(|n| n.get("parameters").cloned());
                                        let node_type = doc
                                            .and_then(|n| n.get("type").and_then(|v| v.as_str()).map(String::from));
                                        let sanitized = match (&node_type, &params) {
                                            (Some(ty), Some(p)) => {
                                                if let Some(schema) = get_schema(ty) {
                                                    match sanitize_parameters(schema, p) {
                                                        Ok(clean) => Some(clean),
                                                        Err(e) => {
                                                            toast.set(Some(e));
                                                            return;
                                                        }
                                                    }
                                                } else {
                                                    params.clone()
                                                }
                                            }
                                            _ => params,
                                        };
                                        if let Some(clean) = sanitized.clone() {
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str())
                                                        == Some(node_id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        obj.insert("parameters".into(), clean);
                                                    }
                                                }
                                            });
                                        }
                                        let step_input = if node_type.as_deref() == Some("apiEndpoint") {
                                            sanitized
                                                .as_ref()
                                                .and_then(sample_from_api_endpoint_parameters)
                                        } else {
                                            None
                                        };
                                        spawn(async move {
                                            let mut body = serde_json::json!({
                                                "nodeId": node_id,
                                                "parameters": sanitized,
                                            });
                                            if let Some(input) = step_input {
                                                if let Some(obj) = body.as_object_mut() {
                                                    obj.insert("input".into(), input);
                                                }
                                            }
                                            match crate::api::post_json(
                                                &format!("/api/workflows/{wid}/execute-node"),
                                                body,
                                            )
                                            .await
                                            {
                                                Ok(v) => {
                                                    if let Some(err) =
                                                        v.get("error").and_then(|e| e.as_str())
                                                    {
                                                        toast.set(Some(err.to_string()));
                                                    } else {
                                                        if let Some(out) = v.get("output") {
                                                            node_output_samples.with_mut(|m| {
                                                                m.insert(node_id.clone(), out.clone());
                                                            });
                                                        }
                                                        toast.set(Some("Step executed".into()));
                                                        details_open.set(true);
                                                    }
                                                }
                                                Err(e) => toast.set(Some(e)),
                                            }
                                        });
                                    },
                                    on_close: move |_| details_open.set(false),
                                    on_param_error: move |msg: String| toast.set(Some(msg)),
                                    on_update: move |upd: NodeUpdate| {
                                        if upd.delete {
                                            let id = upd.id.clone();
                                            doc_nodes.with_mut(|nodes| {
                                                nodes.retain(|n| {
                                                    n.get("id").and_then(|v| v.as_str()) != Some(id.as_str())
                                                });
                                            });
                                            flow_nodes.with_mut(|nodes| {
                                                nodes.retain(|n| n.id != id);
                                            });
                                            flow_edges.with_mut(|edges| {
                                                edges.retain(|e| e.source != id && e.target != id);
                                            });
                                            return;
                                        }
                                        if let Some(name) = &upd.name {
                                            let id = upd.id.clone();
                                            let name = name.clone();
                                            flow_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| n.id == id) {
                                                    n.label = name.clone();
                                                }
                                            });
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str()) == Some(id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        obj.insert(
                                                            "name".into(),
                                                            serde_json::Value::String(name),
                                                        );
                                                    }
                                                }
                                            });
                                        }
                                        if let Some((key, value)) = &upd.parameter_patch {
                                            let id = upd.id.clone();
                                            let key = key.clone();
                                            let value = value.clone();
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str())
                                                        == Some(id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        let params = obj
                                                            .entry("parameters".to_string())
                                                            .or_insert_with(|| {
                                                                serde_json::json!({})
                                                            });
                                                        if let Some(p) = params.as_object_mut() {
                                                            if value.is_null() {
                                                                p.remove(&key);
                                                            } else {
                                                                p.insert(key, value);
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                        if let Some(parameters) = &upd.parameters {
                                            let id = upd.id.clone();
                                            let parameters = parameters.clone();
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str()) == Some(id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        obj.insert("parameters".into(), parameters);
                                                    }
                                                }
                                            });
                                        }
                                        if let Some(settings) = &upd.settings {
                                            let id = upd.id.clone();
                                            let settings = settings.clone();
                                            let disabled = settings
                                                .get("disabled")
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(false);
                                            flow_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| n.id == id) {
                                                    n.data.disabled = disabled;
                                                }
                                            });
                                            doc_nodes.with_mut(|nodes| {
                                                if let Some(n) = nodes.iter_mut().find(|n| {
                                                    n.get("id").and_then(|v| v.as_str()) == Some(id.as_str())
                                                }) {
                                                    if let Some(obj) = n.as_object_mut() {
                                                        obj.insert("settings".into(), settings);
                                                    }
                                                }
                                            });
                                        }
                                    },
                                }
                            }),
                            details_open: details_open(),
                        }
                    }
                }

                if !show_editor && section() == NavSection::Dashboard {
                    DashboardPanel {
                        refresh_key: refresh_key(),
                        on_open_workflow: move |id: String| {
                            selected_id.set(Some(id.clone()));
                            section.set(NavSection::Workflows);
                            editor_open.set(true);
                            reload_workflow(id);
                        },
                        on_navigate: move |s| section.set(s),
                        on_toggle_enabled: move |(id, enabled): (String, bool)| {
                            spawn(async move {
                                let _ = crate::api::put_json(
                                    &format!("/api/workflows/{id}"),
                                    serde_json::json!({ "enabled": enabled }),
                                )
                                .await;
                                refresh_key.with_mut(|k| *k += 1);
                            });
                        },
                    }
                }

                if !show_editor && section() == NavSection::Workflows {
                    WorkflowList {
                        refresh_key: refresh_key(),
                        creating: creating(),
                        exclude_api: true,
                        on_open: move |id: String| {
                            selected_id.set(Some(id.clone()));
                            editor_open.set(true);
                            reload_workflow(id);
                        },
                        on_create: move |_| {
                            creating.set(true);
                            spawn(async move {
                                let body = serde_json::json!({ "name": "My workflow" });
                                match crate::api::post_json("/api/workflows", body).await {
                                    Ok(v) => {
                                        refresh_key.with_mut(|k| *k += 1);
                                        if let Some(id) = v
                                            .get("workflow")
                                            .and_then(|w| w.get("_id").or_else(|| w.get("id")))
                                            .and_then(|x| x.as_str())
                                        {
                                            selected_id.set(Some(id.to_string()));
                                            editor_open.set(true);
                                            reload_workflow(id.to_string());
                                        }
                                    }
                                    Err(e) => toast.set(Some(e)),
                                }
                                creating.set(false);
                            });
                        },
                        on_toggle_enabled: move |(id, enabled): (String, bool)| {
                            spawn(async move {
                                let _ = crate::api::put_json(
                                    &format!("/api/workflows/{id}"),
                                    serde_json::json!({ "enabled": enabled }),
                                )
                                .await;
                                refresh_key.with_mut(|k| *k += 1);
                            });
                        },
                    }
                }

                if !show_editor && section() == NavSection::Apis {
                    ApiProjectsPanel {
                        selected_project_id: api_project_id(),
                        on_selected_project_id_change: Some(EventHandler::new(move |id| {
                            api_project_id.set(id);
                        })),
                        on_open_workflow: move |id: String| {
                            selected_id.set(Some(id.clone()));
                            section.set(NavSection::Apis);
                            editor_open.set(true);
                            reload_workflow(id);
                        },
                    }
                }

                if section() == NavSection::Variables {
                    VariablesPanel {}
                }
                if section() == NavSection::Credentials {
                    CredentialsPanel {}
                }
                if section() == NavSection::Nodes {
                    NodesPanel {
                        can_add: false,
                        on_add_node: None,
                    }
                }
                if section() == NavSection::Runs {
                    div { class: "h-full p-3",
                        div { class: "h-full overflow-hidden rounded-lg border",
                            RunsPanel { refresh_key: refresh_key() }
                        }
                    }
                }
                if section() == NavSection::Insights {
                    div { class: "h-full p-3",
                        div { class: "h-full overflow-hidden rounded-lg border",
                            InsightsPanel { refresh_key: refresh_key() }
                        }
                    }
                }
                if section() == NavSection::Audit {
                    div { class: "h-full p-3",
                        div { class: "h-full overflow-hidden rounded-lg border",
                            AuditPanel {
                                refresh_key: refresh_key(),
                                actor_name: actor_name.clone(),
                            }
                        }
                    }
                }
                if in_settings {
                    SettingsPanel {
                        page: settings_page(),
                        is_admin: role == "admin",
                        on_navigate: move |p| settings_page.set(p),
                    }
                }
            }

            if let Some(msg) = toast() {
                {
                    let variant = toast_variant(&msg);
                    let title = toast_title(variant);
                    rsx! {
                        div {
                            class: "fixed bottom-4 right-4 z-[80] w-[min(100vw-2rem,22rem)] shadow-2xl",
                            onclick: move |_| toast.set(None),
                            Alert {
                                variant,
                                class: Some("bg-card cursor-pointer".into()),
                                if variant == AlertVariant::Destructive {
                                    CircleAlert { class: Some("size-4".into()) }
                                } else if variant == AlertVariant::Success {
                                    CircleCheck { class: Some("size-4".into()) }
                                }
                                AlertTitle { "{title}" }
                                AlertDescription { "{msg}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
