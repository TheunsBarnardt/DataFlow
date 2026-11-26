//! Rust-native management UI (maud HTML). Replaces the React `web/` studio for day-to-day use.

use crate::auth::AuthUser;
use crate::routes::HttpState;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use botflow_workflow_model::{WorkflowDoc, WorkflowEdge, WorkflowNode};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router<HttpState> {
    // Kept under /ui as a lightweight fallback; primary studio is the React SPA at /.
    Router::new()
        .route("/ui", get(dashboard))
        .route("/ui/workflows", get(workflows_page))
        .route("/ui/workflows/new", get(new_workflow_page).post(create_workflow_form))
        .route("/ui/workflows/{id}", get(workflow_page).post(save_workflow_form))
        .route("/ui/workflows/{id}/run", post(run_workflow_form))
        .route("/ui/workflows/{id}/delete", post(delete_workflow_form))
        .route("/ui/runs", get(runs_page))
        .route("/ui/runs/{id}", get(run_page))
        .route("/ui/nodes", get(nodes_page))
}

fn layout(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · BotFlow" }
                style { (PreEscaped(CSS)) }
            }
            body {
                header class="top" {
                    a href="/ui" class="brand" { "BotFlow" }
                    nav {
                        a href="/ui" { "Dashboard" }
                        a href="/ui/workflows" { "Workflows" }
                        a href="/ui/runs" { "Runs" }
                        a href="/ui/nodes" { "Nodes" }
                        a href="/api/v1/health" { "API" }
                    }
                }
                main { (body) }
                footer {
                    "Rust engine · " (env!("CARGO_PKG_VERSION"))
                    " · React studio removed — this is the native UI"
                }
            }
        }
    }
}

const CSS: &str = r#"
:root { --bg:#0f1419; --panel:#1a2332; --text:#e7ecf3; --muted:#9aa7b8; --accent:#3d8bfd; --ok:#3dd68c; --bad:#f07178; --line:#2a3544; }
* { box-sizing:border-box; }
body { margin:0; font:14px/1.45 system-ui,Segoe UI,sans-serif; background:var(--bg); color:var(--text); min-height:100vh; display:flex; flex-direction:column; }
.top { display:flex; gap:1.5rem; align-items:center; padding:.75rem 1.25rem; border-bottom:1px solid var(--line); background:#0c1016; }
.brand { font-weight:700; color:var(--text); text-decoration:none; font-size:1.1rem; }
nav { display:flex; gap:1rem; }
nav a { color:var(--muted); text-decoration:none; }
nav a:hover { color:var(--text); }
main { flex:1; padding:1.25rem; max-width:1100px; width:100%; margin:0 auto; }
footer { padding:.75rem 1.25rem; color:var(--muted); border-top:1px solid var(--line); font-size:12px; }
h1 { font-size:1.35rem; margin:0 0 1rem; }
h2 { font-size:1.05rem; margin:1.5rem 0 .75rem; color:var(--muted); font-weight:600; }
.card { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:1rem; margin-bottom:1rem; }
.grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(160px,1fr)); gap:.75rem; }
.stat { background:var(--panel); border:1px solid var(--line); border-radius:8px; padding:1rem; }
.stat b { display:block; font-size:1.4rem; }
.stat span { color:var(--muted); font-size:12px; }
table { width:100%; border-collapse:collapse; }
th, td { text-align:left; padding:.5rem .4rem; border-bottom:1px solid var(--line); vertical-align:top; }
th { color:var(--muted); font-weight:600; font-size:12px; text-transform:uppercase; letter-spacing:.03em; }
a { color:var(--accent); }
.btn { display:inline-block; background:var(--accent); color:#fff; border:0; border-radius:6px; padding:.45rem .85rem; cursor:pointer; text-decoration:none; font:inherit; }
.btn.secondary { background:transparent; border:1px solid var(--line); color:var(--text); }
.btn.danger { background:var(--bad); }
.btn + .btn, form.inline { margin-left:.35rem; }
form.inline { display:inline; }
label { display:block; margin:.5rem 0 .25rem; color:var(--muted); font-size:12px; }
input, textarea, select { width:100%; background:#0c1016; border:1px solid var(--line); color:var(--text); border-radius:6px; padding:.5rem .6rem; font:inherit; }
textarea { min-height:280px; font-family:ui-monospace,Consolas,monospace; font-size:12px; }
.row { display:flex; gap:.5rem; flex-wrap:wrap; align-items:center; margin-top:1rem; }
.badge { display:inline-block; padding:.1rem .45rem; border-radius:999px; font-size:11px; background:#243044; }
.badge.ok { background:#1a3d2e; color:var(--ok); }
.badge.bad { background:#3d1a1e; color:var(--bad); }
.muted { color:var(--muted); }
pre { white-space:pre-wrap; word-break:break-word; background:#0c1016; padding:.75rem; border-radius:6px; border:1px solid var(--line); font-size:12px; }
.error { color:var(--bad); margin:.5rem 0; }
"#;

fn html_page(title: &str, body: Markup) -> Html<String> {
    Html(layout(title, body).into_string())
}

fn status_badge(status: &str) -> Markup {
    let class = match status {
        "completed" | "succeeded" => "badge ok",
        "failed" | "error" => "badge bad",
        _ => "badge",
    };
    html! { span class=(class) { (status) } }
}

async fn dashboard(State(state): State<HttpState>, _user: AuthUser) -> Html<String> {
    let wf_count = state.app.list_workflows().await.map(|w| w.len()).unwrap_or(0);
    let run_count = state.app.list_runs(None).await.map(|r| r.len()).unwrap_or(0);
    let node_count = state.app.registry.list_metadata().len();
    html_page(
        "Dashboard",
        html! {
            h1 { "Dashboard" }
            div class="grid" {
                div class="stat" { b { (wf_count) } span { "Workflows" } }
                div class="stat" { b { (run_count) } span { "Runs" } }
                div class="stat" { b { (node_count) } span { "Nodes" } }
            }
            div class="card" {
                p { "BotFlow runs entirely on the Rust engine. Use Workflows to author JSON graphs, Runs to inspect execution, Nodes for the catalog." }
                div class="row" {
                    a class="btn" href="/ui/workflows/new" { "New workflow" }
                    a class="btn secondary" href="/ui/runs" { "View runs" }
                }
            }
        },
    )
}

async fn workflows_page(State(state): State<HttpState>, _user: AuthUser) -> Html<String> {
    let list = state.app.list_workflows().await.unwrap_or_default();
    html_page(
        "Workflows",
        html! {
            h1 { "Workflows" }
            div class="row" {
                a class="btn" href="/ui/workflows/new" { "New workflow" }
            }
            div class="card" {
                @if list.is_empty() {
                    p class="muted" { "No workflows yet." }
                } @else {
                    table {
                        thead { tr { th { "Name" } th { "Version" } th { "Enabled" } th { "Id" } } }
                        tbody {
                            @for wf in &list {
                                tr {
                                    td {
                                        a href=(format!("/ui/workflows/{}", wf.id.clone().unwrap_or_default())) {
                                            (wf.name)
                                        }
                                    }
                                    td { (wf.version) }
                                    td { (if wf.enabled { "yes" } else { "no" }) }
                                    td class="muted" { (wf.id.clone().unwrap_or_default()) }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

async fn new_workflow_page(_user: AuthUser) -> Html<String> {
    let demo = serde_json::to_string_pretty(&json!({
        "name": "New workflow",
        "enabled": true,
        "version": 1,
        "nodes": [
            {"id":"t1","type":"manual","name":"Manual","position":{"x":0,"y":0},"parameters":{}},
            {"id":"s1","type":"set","name":"Set","position":{"x":220,"y":0},"parameters":{"values":{"hello":"world"}}}
        ],
        "edges":[{"id":"e1","source":"t1","target":"s1"}]
    }))
    .unwrap_or_else(|_| "{}".into());
    html_page(
        "New workflow",
        html! {
            h1 { "New workflow" }
            form method="post" action="/ui/workflows/new" {
                label { "Workflow JSON" }
                textarea name="json" { (demo) }
                div class="row" {
                    button class="btn" type="submit" { "Create" }
                    a class="btn secondary" href="/ui/workflows" { "Cancel" }
                }
            }
        },
    )
}

#[derive(Deserialize)]
struct JsonForm {
    json: String,
}

async fn create_workflow_form(
    State(state): State<HttpState>,
    _user: AuthUser,
    Form(form): Form<JsonForm>,
) -> Response {
    match serde_json::from_str::<WorkflowDoc>(&form.json) {
        Ok(wf) => match state.app.create_workflow(wf).await {
            Ok(saved) => {
                let id = saved.id.unwrap_or_default();
                Redirect::to(&format!("/ui/workflows/{id}")).into_response()
            }
            Err(e) => form_error("New workflow", &e.to_string(), &form.json, "/ui/workflows/new"),
        },
        Err(e) => form_error("New workflow", &e.to_string(), &form.json, "/ui/workflows/new"),
    }
}

fn form_error(title: &str, err: &str, json: &str, action: &str) -> Response {
    html_page(
        title,
        html! {
            h1 { (title) }
            p class="error" { (err) }
            form method="post" action=(action) {
                label { "Workflow JSON" }
                textarea name="json" { (json) }
                div class="row" { button class="btn" type="submit" { "Retry" } }
            }
        },
    )
    .into_response()
}

async fn workflow_page(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.get_workflow(&id).await {
        Ok(Some(wf)) => {
            let json = serde_json::to_string_pretty(&wf).unwrap_or_else(|_| "{}".into());
            let runs = state
                .app
                .list_runs(Some(&id))
                .await
                .unwrap_or_default();
            html_page(
                &wf.name,
                html! {
                    h1 { (wf.name) }
                    p class="muted" { (format!("id {id} · v{}", wf.version)) }
                    div class="row" {
                        form class="inline" method="post" action=(format!("/ui/workflows/{id}/run")) {
                            button class="btn" type="submit" { "Run" }
                        }
                        form class="inline" method="post" action=(format!("/ui/workflows/{id}/delete")) {
                            button class="btn danger" type="submit" { "Delete" }
                        }
                        a class="btn secondary" href="/ui/workflows" { "Back" }
                    }
                    h2 { "Definition" }
                    form method="post" action=(format!("/ui/workflows/{id}")) {
                        textarea name="json" { (json) }
                        div class="row" { button class="btn" type="submit" { "Save" } }
                    }
                    h2 { "Recent runs" }
                    div class="card" {
                        @if runs.is_empty() {
                            p class="muted" { "No runs yet." }
                        } @else {
                            table {
                                thead { tr { th { "Status" } th { "Started" } th { "Id" } } }
                                tbody {
                                    @for r in runs.iter().take(20) {
                                        tr {
                                            td { (status_badge(&format!("{:?}", r.status).to_lowercase().replace("succeeded","completed"))) }
                                            td { (r.started_at.to_rfc3339()) }
                                            td {
                                                a href=(format!("/ui/runs/{}", r.id.clone().unwrap_or_default())) {
                                                    (r.id.clone().unwrap_or_default())
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            )
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "workflow not found").into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn save_workflow_form(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Form(form): Form<JsonForm>,
) -> Response {
    match serde_json::from_str::<WorkflowDoc>(&form.json) {
        Ok(wf) => match state.app.update_workflow(&id, wf).await {
            Ok(Some(_)) => Redirect::to(&format!("/ui/workflows/{id}")).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
            Err(e) => form_error("Edit workflow", &e.to_string(), &form.json, &format!("/ui/workflows/{id}")),
        },
        Err(e) => form_error("Edit workflow", &e.to_string(), &form.json, &format!("/ui/workflows/{id}")),
    }
}

async fn run_workflow_form(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state
        .app
        .run_workflow(&id, json!({ "from": "ui" }))
        .await
    {
        Ok(run) => {
            let rid = run.id.unwrap_or_default();
            Redirect::to(&format!("/ui/runs/{rid}")).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_workflow_form(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    let _ = state.app.delete_workflow(&id).await;
    Redirect::to("/ui/workflows").into_response()
}

async fn runs_page(State(state): State<HttpState>, _user: AuthUser) -> Html<String> {
    let runs = state.app.list_runs(None).await.unwrap_or_default();
    html_page(
        "Runs",
        html! {
            h1 { "Runs" }
            div class="card" {
                table {
                    thead { tr { th { "Status" } th { "Workflow" } th { "Started" } th { "Id" } } }
                    tbody {
                        @for r in &runs {
                            tr {
                                td {
                                    (status_badge(match r.status {
                                        botflow_workflow_model::RunStatus::Succeeded => "completed",
                                        botflow_workflow_model::RunStatus::Failed => "failed",
                                        botflow_workflow_model::RunStatus::Running => "running",
                                        botflow_workflow_model::RunStatus::Cancelled => "cancelled",
                                        botflow_workflow_model::RunStatus::Queued => "queued",
                                        botflow_workflow_model::RunStatus::Waiting => "waiting",
                                        botflow_workflow_model::RunStatus::Retrying => "retrying",
                                        botflow_workflow_model::RunStatus::Created => "created",
                                    }))
                                }
                                td {
                                    a href=(format!("/ui/workflows/{}", r.workflow_id)) { (r.workflow_id) }
                                }
                                td { (r.started_at.to_rfc3339()) }
                                td {
                                    a href=(format!("/ui/runs/{}", r.id.clone().unwrap_or_default())) {
                                        (r.id.clone().unwrap_or_default())
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

async fn run_page(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.get_run(&id).await {
        Ok(Some(run)) => {
            let ui = run.to_ui_json();
            let pretty = serde_json::to_string_pretty(&ui).unwrap_or_default();
            html_page(
                "Run",
                html! {
                    h1 { "Run " (id.clone()) }
                    p {
                        (status_badge(ui.get("status").and_then(|v| v.as_str()).unwrap_or("")))
                        " · workflow "
                        a href=(format!("/ui/workflows/{}", run.workflow_id)) { (run.workflow_id) }
                    }
                    div class="row" {
                        a class="btn secondary" href="/ui/runs" { "All runs" }
                    }
                    h2 { "Details" }
                    pre { (pretty) }
                },
            )
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "run not found").into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn nodes_page(State(state): State<HttpState>, _user: AuthUser) -> Html<String> {
    let mut nodes = state.app.registry.list_metadata();
    nodes.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.display_name.cmp(&b.display_name))
    });
    html_page(
        "Nodes",
        html! {
            h1 { "Node catalog" }
            p class="muted" { (format!("{} registered executors", nodes.len())) }
            div class="card" {
                table {
                    thead { tr { th { "Type" } th { "Label" } th { "Category" } th { "Trigger" } } }
                    tbody {
                        @for n in &nodes {
                            tr {
                                td { code { (n.type_name) } }
                                td { (n.display_name) }
                                td { (n.category) }
                                td { (if n.is_trigger { "yes" } else { "" }) }
                            }
                        }
                    }
                }
            }
        },
    )
}

// silence unused imports if workflow builders unused
#[allow(dead_code)]
fn _types(_: WorkflowNode, _: WorkflowEdge) {}
