use crate::auth::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, patch, post},
    Json, Router,
};
use botflow_application_services::AppState;
use botflow_security::JwtAuth;
use botflow_workflow_model::WorkflowDoc;
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    sync::{LazyLock, Mutex},
    time::Duration,
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct HttpState {
    pub app: AppState,
    pub jwt: JwtAuth,
    pub auth_disabled: bool,
}

pub fn router(app: AppState, cfg: crate::HttpConfig) -> Router {
    let state = HttpState {
        app,
        jwt: cfg.jwt,
        auth_disabled: cfg.auth_disabled,
    };
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/health", get(health))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/nodes", get(list_nodes))
        .route("/api/v1/workflows", get(list_workflows).post(create_workflow))
        .route("/api/workflows", get(list_workflows).post(create_workflow))
        .route(
            "/api/v1/workflows/{id}",
            get(get_workflow).put(update_workflow).delete(delete_workflow),
        )
        .route(
            "/api/workflows/{id}",
            get(get_workflow).put(update_workflow).delete(delete_workflow),
        )
        .route("/api/v1/workflows/{id}/validate", post(validate_workflow))
        .route("/api/v1/workflows/{id}/run", post(run_workflow))
        .route("/api/workflows/{id}/run", post(run_workflow))
        .route(
            "/api/v1/workflows/{id}/execute-node",
            post(execute_node),
        )
        .route("/api/workflows/{id}/execute-node", post(execute_node))
        .route("/api/v1/runs", get(list_runs))
        .route("/api/runs", get(list_runs))
        .route("/api/v1/runs/{id}", get(get_run))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/v1/runs/{id}/cancel", post(cancel_run))
        .route("/api/runs", axum::routing::delete(clear_runs))
        .route("/api/v1/events", get(sse_events))
        .route("/api/events", get(sse_events))
        .route("/api/v1/webhooks/{path}", post(webhook_ingress))
        .route("/hooks/{path}", post(webhook_ingress))
        .route("/api/credentials", get(list_credentials).post(create_credential))
        .route("/api/v1/credentials", get(list_credentials).post(create_credential))
        .route(
            "/api/credentials/test",
            post(test_credential_connection),
        )
        .route(
            "/api/v1/credentials/test",
            post(test_credential_connection),
        )
        .route(
            "/api/credentials/{id}",
            get(get_credential)
                .put(update_credential)
                .delete(delete_credential),
        )
        .route("/api/variables", get(list_variables).post(upsert_variable))
        .route("/api/v1/variables", get(list_variables).post(upsert_variable))
        .route(
            "/api/variables/{id}",
            axum::routing::put(upsert_variable).delete(delete_variable_by_id),
        )
        .route("/api/webhooks", get(list_webhooks).post(upsert_webhook))
        .route("/api/v1/webhooks", get(list_webhooks).post(upsert_webhook))
        .route("/api/schedules", get(list_schedules).post(upsert_schedule))
        .route("/api/v1/schedules", get(list_schedules).post(upsert_schedule))
        .route("/api/audit", get(list_audit))
        .route("/api/v1/audit", get(list_audit))
        .route("/api/insights", get(insights))
        .route("/api/v1/insights", get(insights))
        .route("/api/me", get(me).patch(patch_me))
        .route("/api/users", get(list_users_handler).post(create_user_handler))
        .route("/api/users/{id}", patch(patch_user_handler))
        .route(
            "/api/settings/authentication",
            get(get_authentication_settings_handler).patch(patch_authentication_settings_handler),
        )
        .route(
            "/api/settings/api-keys",
            get(list_api_keys_handler).post(create_api_key_handler),
        )
        .route(
            "/api/settings/api-keys/{key_id}",
            patch(patch_api_key_handler).delete(delete_api_key_handler),
        )
        .route(
            "/api/api-projects",
            get(list_api_projects).post(create_api_project),
        )
        .route(
            "/api/api-projects/{id}",
            get(get_api_project)
                .put(update_api_project)
                .delete(delete_api_project),
        )
        .route(
            "/api/api-projects/{id}/endpoints",
            post(create_api_endpoint),
        )
        .with_state(state)
}

#[derive(Default, Clone)]
struct LocalProfileOverlay {
    name: Option<String>,
    company_name: Option<String>,
}

static LOCAL_PROFILE: LazyLock<Mutex<LocalProfileOverlay>> =
    LazyLock::new(|| Mutex::new(LocalProfileOverlay::default()));

fn default_local_user() -> Value {
    json!({
        "id": "local",
        "name": "Local",
        "email": "local@botflow",
        "username": "local",
        "role": "admin",
        "companyName": "",
        "status": "active",
    })
}

fn apply_local_overlay(mut user: Value) -> Value {
    let overlay = LOCAL_PROFILE.lock().ok();
    if let Some(overlay) = overlay {
        if let Some(name) = overlay.name.as_deref().filter(|s| !s.is_empty()) {
            user["name"] = json!(name);
        }
        if let Some(company) = overlay.company_name.as_ref() {
            user["companyName"] = json!(company);
        }
    }
    user
}

const LOCAL_USER_ID: &str = "local";

async fn resolve_me_user(state: &HttpState, user: &AuthUser) -> Value {
    if state.auth_disabled {
        if let Ok(Some(u)) = botflow_storage::find_user_by_id(&state.app.db, LOCAL_USER_ID).await {
            return apply_local_overlay(u);
        }
        return apply_local_overlay(default_local_user());
    }
    if let Ok(Some(u)) = botflow_storage::find_user_by_identity(&state.app.db, &user.sub).await {
        return u;
    }
    json!({
        "id": user.sub,
        "name": user.sub,
        "email": format!("{}@botflow", user.sub),
        "username": user.sub,
        "role": user.role,
        "companyName": "",
        "status": "active",
    })
}

async fn me(State(state): State<HttpState>, user: AuthUser) -> Response {
    let u = resolve_me_user(&state, &user).await;
    Json(json!({ "user": u })).into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchMeBody {
  name: Option<String>,
  company_name: Option<String>,
}

async fn patch_me(
    State(state): State<HttpState>,
    user: AuthUser,
    Json(body): Json<PatchMeBody>,
) -> Response {
    let name = body.name.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let company = body.company_name.as_deref().map(str::trim);
    if name.is_none() && company.is_none() {
        let u = resolve_me_user(&state, &user).await;
        return Json(json!({ "user": u })).into_response();
    }

    if state.auth_disabled {
        if let Ok(mut overlay) = LOCAL_PROFILE.lock() {
            if let Some(n) = name {
                overlay.name = Some(n.to_string());
            }
            if let Some(c) = company {
                overlay.company_name = Some(c.to_string());
            }
        }
        let _ = botflow_storage::upsert_local_user_profile(
            &state.app.db,
            LOCAL_USER_ID,
            name,
            company,
        )
        .await;
        let u = resolve_me_user(&state, &user).await;
        return Json(json!({ "user": u })).into_response();
    }

    let identity = user.sub.clone();
    let mut set = mongodb::bson::Document::new();
    if let Some(n) = name {
        set.insert("name", n);
    }
    if let Some(c) = company {
        set.insert("companyName", c);
    }
    set.insert("updatedAt", chrono::Utc::now().to_rfc3339());
    match botflow_storage::find_user_by_identity(&state.app.db, &identity).await {
        Ok(Some(existing)) => {
            let uid = existing
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&identity)
                .to_string();
            match botflow_storage::update_user_by_id(&state.app.db, &uid, set).await {
                Ok(Some(u)) => Json(json!({ "user": u })).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
                Err(e) => error_response(e),
            }
        }
        Ok(None) => {
            let u = resolve_me_user(&state, &user).await;
            Json(json!({ "user": u })).into_response()
        }
        Err(e) => error_response(e),
    }
}

async fn list_users_handler(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_users(&state.app.db).await {
        Ok(mut users) => {
            if users.is_empty() && state.auth_disabled {
                users.push(apply_local_overlay(default_local_user()));
            }
            Json(json!({ "users": users })).into_response()
        }
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserBody {
    email: String,
    password: String,
    name: String,
    username: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    company_name: Option<String>,
}

async fn create_user_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<CreateUserBody>,
) -> Response {
    let email = body.email.trim();
    let name = body.name.trim();
    let username = body.username.trim();
    if email.is_empty() || body.password.is_empty() || name.is_empty() || username.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Missing required fields" })),
        )
            .into_response();
    }
    let role = body
        .role
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("editor");
    let company = body.company_name.as_deref().unwrap_or("");
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let doc = mongodb::bson::doc! {
        "id": &id,
        "email": email,
        "name": name,
        "username": username.to_lowercase(),
        "displayUsername": username,
        "role": role,
        "companyName": company,
        "status": "active",
        "password": botflow_storage::hash_local_password(&body.password),
        "createdAt": &now,
        "updatedAt": &now,
    };
    match botflow_storage::insert_user(&state.app.db, doc).await {
        Ok(user) => (StatusCode::CREATED, Json(json!({ "user": user }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchUserBody {
    role: Option<String>,
    status: Option<String>,
    company_name: Option<String>,
    name: Option<String>,
}

async fn patch_user_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<PatchUserBody>,
) -> Response {
    let mut set = mongodb::bson::Document::new();
    if let Some(role) = body.role.as_deref().filter(|s| !s.is_empty()) {
        set.insert("role", role);
    }
    if let Some(status) = body.status.as_deref().filter(|s| !s.is_empty()) {
        set.insert("status", status);
    }
    if let Some(company) = body.company_name.as_ref() {
        set.insert("companyName", company);
    }
    if let Some(name) = body.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        set.insert("name", name);
    }
    if set.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "No updates" })),
        )
            .into_response();
    }
    set.insert("updatedAt", chrono::Utc::now().to_rfc3339());

    if id == "local" && state.auth_disabled {
        if let Ok(mut overlay) = LOCAL_PROFILE.lock() {
            if let Some(name) = body.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                overlay.name = Some(name.to_string());
            }
            if let Some(company) = body.company_name.as_ref() {
                overlay.company_name = Some(company.clone());
            }
        }
        let _ = botflow_storage::upsert_local_user_profile(
            &state.app.db,
            "local",
            body.name.as_deref().map(str::trim),
            body.company_name.as_deref(),
        )
        .await;
        let user = apply_local_overlay(default_local_user());
        return Json(json!({ "user": user })).into_response();
    }

    match botflow_storage::update_user_by_id(&state.app.db, &id, set).await {
        Ok(Some(user)) => Json(json!({ "user": user })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_authentication_settings_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
) -> Response {
    match botflow_storage::get_authentication_settings(&state.app.db).await {
        Ok(settings) => {
            let public = botflow_storage::public_authentication_settings(&settings);
            Json(json!({ "settings": public })).into_response()
        }
        Err(e) => error_response(e),
    }
}

async fn list_api_keys_handler(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_instance_api_keys(&state.app.db).await {
        Ok(api_keys) => {
            let total = api_keys.len();
            Json(json!({ "apiKeys": api_keys, "total": total })).into_response()
        }
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyBody {
    name: Option<String>,
    expires_in: Option<i64>,
    metadata: Option<Value>,
}

async fn create_api_key_handler(
    State(state): State<HttpState>,
    user: AuthUser,
    Json(body): Json<CreateApiKeyBody>,
) -> Response {
    let name = body.name.unwrap_or_default();
    match botflow_storage::create_instance_api_key(
        &state.app.db,
        botflow_storage::CreateInstanceApiKeyInput {
            name,
            user_id: Some(user.sub),
            expires_in_secs: body.expires_in,
            metadata: body.metadata,
        },
    )
    .await
    {
        Ok(created) => {
            let mut key = created.public;
            if let Some(obj) = key.as_object_mut() {
                obj.insert("key".into(), Value::String(created.secret));
            }
            Json(json!({ "key": key })).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("name required") {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            } else {
                error_response(e)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchApiKeyBody {
    enabled: Option<bool>,
    name: Option<String>,
    metadata: Option<Value>,
}

async fn patch_api_key_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(key_id): Path<String>,
    Json(body): Json<PatchApiKeyBody>,
) -> Response {
    match botflow_storage::update_instance_api_key(
        &state.app.db,
        &key_id,
        body.enabled,
        body.name.as_deref(),
        body.metadata,
    )
    .await
    {
        Ok(Some(key)) => Json(json!({ "key": key })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("No values to update") {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            } else {
                error_response(e)
            }
        }
    }
}

async fn delete_api_key_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(key_id): Path<String>,
) -> Response {
    match botflow_storage::delete_instance_api_key(&state.app.db, &key_id).await {
        Ok(true) => Json(json!({ "success": true })).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn patch_authentication_settings_handler(
    State(state): State<HttpState>,
    _user: AuthUser,
    body: Option<Json<Value>>,
) -> Response {
    let incoming = body.map(|b| b.0).unwrap_or_else(|| json!({}));
    match botflow_storage::get_authentication_settings(&state.app.db).await {
        Ok(current) => {
            let mut next = current;
            if let Some(mfa) = incoming.get("mfaEnforced").and_then(|v| v.as_bool()) {
                next["mfaEnforced"] = json!(mfa);
            }
            if let Some(sso) = incoming.get("sso") {
                deep_merge_json(&mut next["sso"], sso);
            }
            if let Some(ldap) = incoming.get("ldap") {
                deep_merge_json(&mut next["ldap"], ldap);
            }
            match botflow_storage::save_authentication_settings(&state.app.db, next.clone()).await {
                Ok(saved) => {
                    let public = botflow_storage::public_authentication_settings(&saved);
                    Json(json!({ "settings": public })).into_response()
                }
                Err(e) => error_response(e),
            }
        }
        Err(e) => error_response(e),
    }
}

fn deep_merge_json(base: &mut Value, patch: &Value) {
    let Value::Object(base_map) = base else {
        return;
    };
    let Some(patch_map) = patch.as_object() else {
        return;
    };
    for (key, value) in patch_map {
        if let Some(existing) = base_map.get_mut(key) {
            if existing.is_object() && value.is_object() {
                deep_merge_json(existing, value);
                continue;
            }
        }
        base_map.insert(key.clone(), value.clone());
    }
}

async fn delete_variable_by_id(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::delete_variable(&state.app.db, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "botflow-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

async fn login(State(state): State<HttpState>, Json(body): Json<LoginReq>) -> Response {
    let expected_user = std::env::var("BOTFLOW_ADMIN_USER").unwrap_or_else(|_| "admin".into());
    let expected_pass =
        std::env::var("BOTFLOW_ADMIN_PASSWORD").unwrap_or_else(|_| "P@ssw0rd".into());
    if body.username != expected_user || body.password != expected_pass {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid credentials" })),
        )
            .into_response();
    }
    match state.jwt.issue(&body.username, "admin") {
        Ok(token) => Json(json!({ "token": token, "role": "admin" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_nodes(State(state): State<HttpState>, _user: AuthUser) -> Json<Value> {
    let nodes: Vec<Value> = state
        .app
        .registry
        .list_metadata()
        .into_iter()
        .map(|m| {
            json!({
                "type": m.type_name,
                "label": m.display_name,
                "category": m.category,
                "description": m.description,
                "isTrigger": m.is_trigger,
                "version": m.version,
            })
        })
        .collect();
    Json(json!({ "nodes": nodes }))
}

async fn list_workflows(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match state.app.list_workflows().await {
        Ok(list) => Json(json!({ "workflows": list })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.get_workflow(&id).await {
        Ok(Some(wf)) => Json(json!({ "workflow": wf })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn create_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(wf): Json<WorkflowDoc>,
) -> Response {
    match state.app.create_workflow(wf).await {
        Ok(wf) => (StatusCode::CREATED, Json(json!({ "workflow": wf }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn update_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(wf): Json<WorkflowDoc>,
) -> Response {
    match state.app.update_workflow(&id, wf).await {
        Ok(Some(wf)) => Json(json!({ "workflow": wf })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.delete_workflow(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn validate_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.get_workflow(&id).await {
        Ok(Some(wf)) => {
            let result = state.app.validate(&wf).await;
            Json(json!({ "ok": result.ok(), "issues": result.issues })).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct RunBody {
    /// Missing / null → `{}` so triggers are not fed `[null]`.
    #[serde(default)]
    input: Option<Value>,
}

async fn run_workflow(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    body: Option<Json<RunBody>>,
) -> Response {
    let input = match body {
        None => json!({ "from": "api" }),
        Some(Json(b)) => match b.input {
            None | Some(Value::Null) => json!({}),
            Some(v) => v,
        },
    };
    match state.app.run_workflow(&id, input).await {
        Ok(run) => Json(json!({ "run": run.to_ui_json() })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct ExecuteNodeBody {
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    parameters: Option<Value>,
}

async fn execute_node(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ExecuteNodeBody>,
) -> Response {
    match state
        .app
        .execute_single_node(&id, &body.node_id, body.input, body.parameters)
        .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct RunsQuery {
    #[serde(default, rename = "workflowId", alias = "workflow_id")]
    workflow_id: Option<String>,
}

async fn list_runs(
    State(state): State<HttpState>,
    _user: AuthUser,
    Query(q): Query<RunsQuery>,
) -> Response {
    match state.app.list_runs(q.workflow_id.as_deref()).await {
        // Wrap to match TypeScript UI: { runs: [...] }
        Ok(list) => {
            let runs: Vec<Value> = list.iter().map(|r| r.to_ui_json()).collect();
            Json(json!({ "runs": runs })).into_response()
        }
        Err(e) => error_response(e),
    }
}

async fn get_run(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.get_run(&id).await {
        Ok(Some(run)) => Json(json!({ "run": run.to_ui_json() })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn cancel_run(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match state.app.cancel_run(&id).await {
        Ok(true) => Json(json!({ "cancelled": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "run not active" })),
        )
            .into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct SseQuery {
    run_id: Option<String>,
}

async fn sse_events(
    State(state): State<HttpState>,
    _user: AuthUser,
    Query(q): Query<SseQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.events.subscribe();
    let run_filter = q.run_id;
    let stream = BroadcastStream::new(rx).filter_map(move |msg| {
        let Ok(event) = msg else { return None };
        if let Some(rid) = &run_filter {
            if &event.run_id != rid {
                return None;
            }
        }
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".into());
        Some(Ok(Event::default().event("run").data(data)))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn webhook_ingress(
    State(state): State<HttpState>,
    Path(path): Path<String>,
    body: Option<Json<Value>>,
) -> Response {
    // Find workflow with a webhook node whose parameters.path matches
    let Ok(workflows) = state.app.list_workflows().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "db error" })),
        )
            .into_response();
    };
    let payload = body.map(|b| b.0).unwrap_or(json!({}));
    for wf in workflows {
        let matches = wf.nodes.iter().any(|n| {
            n.node_type == "webhook"
                && n.parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|p| p == path || p.trim_matches('/') == path.trim_matches('/'))
                    .unwrap_or(false)
        });
        if matches {
            if let Some(id) = &wf.id {
                return match state
                    .app
                    .run_workflow(
                        id,
                        json!({ "webhook": { "path": path, "body": payload } }),
                    )
                    .await
                {
                    Ok(run) => Json(run).into_response(),
                    Err(e) => error_response(e),
                };
            }
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "no webhook for path" })),
    )
        .into_response()
}

fn error_response(e: impl std::fmt::Display) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": e.to_string() })),
    )
        .into_response()
}

async fn clear_runs(
    State(state): State<HttpState>,
    _user: AuthUser,
    Query(q): Query<RunsQuery>,
) -> Response {
    match botflow_storage::delete_runs(&state.app.db, q.workflow_id.as_deref()).await {
        Ok(n) => Json(json!({ "ok": true, "deleted": n })).into_response(),
        Err(e) => error_response(e),
    }
}

fn credential_public_json(c: &botflow_storage::CredentialDoc) -> Value {
    json!({
        "id": c.id.clone().unwrap_or_default(),
        "name": c.name,
        "type": c.cred_type,
    })
}

#[derive(Deserialize)]
struct CredTestBody {
    #[serde(rename = "type")]
    cred_type: String,
    data: Value,
}

async fn test_credential_connection(
    _user: AuthUser,
    Json(body): Json<CredTestBody>,
) -> Response {
    let ty = body.cred_type.trim().to_lowercase();
    if ty == "microsoftsql" || ty == "mssql" {
        let result = botflow_integrations::test_mssql_connection(&body.data).await;
        return Json(json!({
            "ok": result.ok,
            "message": result.message,
            "databases": result.databases,
        }))
        .into_response();
    }
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "ok": false,
            "error": format!("connection test not supported for credential type '{}'", body.cred_type),
        })),
    )
        .into_response()
}

async fn list_credentials(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_credentials(&state.app.db).await {
        Ok(list) => {
            let credentials: Vec<Value> = list.iter().map(credential_public_json).collect();
            Json(json!({ "credentials": credentials })).into_response()
        }
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct CredBody {
    name: String,
    #[serde(rename = "type")]
    cred_type: String,
    data: Value,
}

async fn create_credential(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<CredBody>,
) -> Response {
    let enc = match state.app.secrets.encrypt_json(&body.data) {
        Ok(s) => s,
        Err(e) => return error_response(e),
    };
    let doc = botflow_storage::CredentialDoc {
        id: None,
        name: body.name,
        cred_type: body.cred_type,
        data: enc,
        created_at: None,
        updated_at: None,
    };
    match botflow_storage::upsert_credential(&state.app.db, doc).await {
        Ok(c) => Json(json!({ "credential": credential_public_json(&c) })).into_response(),
        Err(e) => error_response(e),
    }
}

fn decrypt_credential_data(secrets: &botflow_security::SecretBox, blob: &str) -> Value {
    if let Ok(v) = secrets.decrypt_json(blob) {
        return v;
    }
    serde_json::from_str(blob).unwrap_or_else(|_| json!({}))
}

async fn get_credential(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::get_credential(&state.app.db, &id).await {
        Ok(Some(c)) => {
            let data = decrypt_credential_data(&state.app.secrets, &c.data);
            Json(json!({
                "credential": {
                    "id": c.id.clone().unwrap_or_default(),
                    "name": c.name,
                    "type": c.cred_type,
                    "data": data,
                }
            }))
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct CredUpdateBody {
    name: String,
    #[serde(default)]
    data: Option<Value>,
}

async fn update_credential(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<CredUpdateBody>,
) -> Response {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "name required" })),
        )
            .into_response();
    }
    let existing = match botflow_storage::get_credential(&state.app.db, &id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response();
        }
        Err(e) => return error_response(e),
    };
    let data = if let Some(payload) = body.data {
        match state.app.secrets.encrypt_json(&payload) {
            Ok(s) => s,
            Err(e) => return error_response(e),
        }
    } else {
        existing.data
    };
    let doc = botflow_storage::CredentialDoc {
        id: Some(id),
        name,
        cred_type: existing.cred_type,
        data,
        created_at: existing.created_at,
        updated_at: None,
    };
    match botflow_storage::upsert_credential(&state.app.db, doc).await {
        Ok(c) => Json(json!({ "credential": credential_public_json(&c) })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_credential(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::delete_credential(&state.app.db, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_variables(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_variables(&state.app.db).await {
        Ok(list) => Json(json!({ "variables": list })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct VarBody {
    key: String,
    value: Value,
}

async fn upsert_variable(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<VarBody>,
) -> Response {
    let doc = botflow_storage::VariableDoc {
        id: None,
        key: body.key,
        value: body.value,
    };
    match botflow_storage::upsert_variable(&state.app.db, doc).await {
        Ok(v) => Json(json!({ "variable": v })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_webhooks(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_webhooks(&state.app.db).await {
        Ok(list) => Json(json!({ "webhooks": list })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhBody {
    path: String,
    workflow_id: String,
    #[serde(default)]
    method: Option<String>,
}

async fn upsert_webhook(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<WhBody>,
) -> Response {
    let doc = botflow_storage::WebhookDoc {
        id: None,
        path: body.path,
        workflow_id: body.workflow_id,
        method: body.method.unwrap_or_else(|| "POST".into()),
    };
    match botflow_storage::upsert_webhook(&state.app.db, doc).await {
        Ok(w) => Json(json!({ "webhook": w })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_schedules(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_schedules(&state.app.db).await {
        Ok(list) => Json(json!({ "schedules": list })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchedBody {
    workflow_id: String,
    cron: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

async fn upsert_schedule(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<SchedBody>,
) -> Response {
    let doc = botflow_storage::ScheduleDoc {
        id: None,
        workflow_id: body.workflow_id,
        cron: body.cron,
        enabled: body.enabled,
        next_run_at: None,
    };
    match botflow_storage::upsert_schedule(&state.app.db, doc).await {
        Ok(s) => Json(json!({ "schedule": s })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_audit(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_audit(&state.app.db, 100).await {
        Ok(list) => Json(json!({ "events": list })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
struct InsightsQuery {
    #[serde(default = "default_insights_days")]
    days: i64,
}

fn default_insights_days() -> i64 {
    7
}

async fn insights(
    State(state): State<HttpState>,
    _user: AuthUser,
    Query(q): Query<InsightsQuery>,
) -> Response {
    match state.app.list_runs_for_insights().await {
        Ok(runs) => Json(crate::insights::build_insights(&runs, q.days)).into_response(),
        Err(e) => error_response(e),
    }
}

fn slugify(raw: &str) -> String {
    let s: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = s.trim_matches('-');
    trimmed.chars().take(64).collect()
}

fn random_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}{:x}", uuid_ish())
}

fn uuid_ish() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().hash(&mut h);
    h.finish()
}

fn short_id(n: usize) -> String {
    format!("{:x}", uuid_ish())
        .chars()
        .take(n)
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProjectBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    auth: Option<Value>,
    #[serde(default)]
    schemas: Option<Value>,
}

async fn list_api_projects(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_api_projects(&state.app.db).await {
        Ok(projects) => Json(json!({ "projects": projects })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_api_project(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::get_api_project(&state.app.db, &id).await {
        Ok(Some(project)) => {
            let endpoints =
                match botflow_storage::list_api_endpoints_for_project(&state.app.db, &id).await {
                    Ok(v) => v,
                    Err(e) => return error_response(e),
                };
            let workflows =
                match botflow_storage::list_api_endpoint_workflows(&state.app.db, &id).await {
                    Ok(v) => v,
                    Err(e) => return error_response(e),
                };
            Json(json!({
                "project": project,
                "endpoints": endpoints,
                "workflows": workflows,
            }))
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn create_api_project(
    State(state): State<HttpState>,
    user: AuthUser,
    Json(body): Json<ApiProjectBody>,
) -> Response {
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("New API")
        .to_string();
    let mut slug = slugify(body.slug.as_deref().unwrap_or(&name));
    if slug.is_empty() {
        slug = format!("api-{}", short_id(8));
    }
    if let Ok(Some(_)) = botflow_storage::find_api_project_by_slug(&state.app.db, &slug).await {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": format!("Slug '{slug}' already exists") })),
        )
            .into_response();
    }
    let now = chrono::Utc::now().to_rfc3339();
    let actor = user.sub.clone();
    let mut auth = body.auth.unwrap_or_else(|| {
        json!({
            "mode": "jwt",
            "jwtSecret": random_secret(),
        })
    });
    if auth.get("mode").and_then(|v| v.as_str()) == Some("jwt")
        && auth
            .get("jwtSecret")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .is_empty()
    {
        auth["jwtSecret"] = json!(random_secret());
    }
    let doc = json!({
        "name": name,
        "slug": slug,
        "description": body.description.unwrap_or_default(),
        "version": body.version.unwrap_or_else(|| "v1".into()),
        "enabled": body.enabled.unwrap_or(true),
        "auth": auth,
        "schemas": body.schemas.unwrap_or_else(|| json!({})),
        "createdBy": actor,
        "updatedBy": user.sub.clone(),
        "createdAt": now,
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    });
    match botflow_storage::insert_api_project(&state.app.db, doc).await {
        Ok(project) => Json(json!({ "project": project })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn update_api_project(
    State(state): State<HttpState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ApiProjectBody>,
) -> Response {
    let existing = match botflow_storage::get_api_project(&state.app.db, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response()
        }
        Err(e) => return error_response(e),
    };
    let mut slug = existing
        .get("slug")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(raw) = body.slug.as_deref() {
        slug = slugify(raw);
        if slug.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid slug" })),
            )
                .into_response();
        }
        if let Ok(Some(clash)) =
            botflow_storage::find_api_project_by_slug(&state.app.db, &slug).await
        {
            let clash_id = clash.get("_id").and_then(|v| v.as_str()).unwrap_or("");
            if clash_id != id {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": format!("Slug '{slug}' already exists") })),
                )
                    .into_response();
            }
        }
    }
    let name = body
        .name
        .unwrap_or_else(|| {
            existing
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("New API")
                .to_string()
        });
    let description = body.description.unwrap_or_else(|| {
        existing
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    });
    let version = body.version.unwrap_or_else(|| {
        existing
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("v1")
            .to_string()
    });
    let enabled = body.enabled.unwrap_or_else(|| {
        existing
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });
    let mut set = json!({
        "name": name,
        "slug": slug,
        "description": description,
        "version": version,
        "enabled": enabled,
        "updatedBy": user.sub.clone(),
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    });
    if let Some(auth) = body.auth {
        set["auth"] = auth;
    }
    if let Some(schemas) = body.schemas {
        set["schemas"] = schemas;
    }
    match botflow_storage::update_api_project(&state.app.db, &id, set).await {
        Ok(Some(project)) => Json(json!({ "project": project })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_api_project(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    let existing = match botflow_storage::get_api_project(&state.app.db, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response()
        }
        Err(e) => return error_response(e),
    };
    let _ = existing;
    // Delete endpoint workflows for this project
    if let Ok(wfs) = botflow_storage::list_api_endpoint_workflows(&state.app.db, &id).await {
        for wf in wfs {
            if let Some(wid) = wf.id.as_deref() {
                let _ = state.app.delete_workflow(wid).await;
            }
        }
    }
    let _ = botflow_storage::delete_api_endpoints_for_project(&state.app.db, &id).await;
    match botflow_storage::delete_api_project(&state.app.db, &id).await {
        Ok(true) => Json(json!({ "ok": true })).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndpointBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    auth: Option<String>,
    #[serde(default)]
    summary: Option<String>,
}

async fn create_api_endpoint(
    State(state): State<HttpState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<EndpointBody>,
) -> Response {
    let project = match botflow_storage::get_api_project(&state.app.db, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response()
        }
        Err(e) => return error_response(e),
    };
    let method = body
        .method
        .as_deref()
        .unwrap_or("GET")
        .trim()
        .to_uppercase();
    let path = body
        .path
        .as_deref()
        .unwrap_or("")
        .trim()
        .trim_matches('/')
        .to_string();
    let path = if path.is_empty() {
        format!("endpoint-{}", short_id(6))
    } else {
        path
    };
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{method} {path}"));
    let default_auth = project
        .pointer("/auth/mode")
        .and_then(|v| v.as_str())
        .filter(|&m| m == "jwt")
        .unwrap_or("none");
    let auth = body.auth.as_deref().unwrap_or(default_auth);
    let tag = body
        .tag
        .clone()
        .unwrap_or_else(|| path.split('/').next().unwrap_or("default").to_string());
    let summary = body.summary.clone().unwrap_or_else(|| name.clone());
    let trigger_id = format!("n_{}", short_id(8));
    let respond_id = format!("n_{}", short_id(8));
    let nodes = vec![
        botflow_workflow_model::WorkflowNode {
            id: trigger_id.clone(),
            node_type: "apiEndpoint".into(),
            name: "API Endpoint".into(),
            position: botflow_workflow_model::Position { x: 80.0, y: 120.0 },
            parameters: json!({
                "path": path,
                "method": method,
                "auth": auth,
                "tag": tag,
                "summary": summary,
            }),
            settings: None,
        },
        botflow_workflow_model::WorkflowNode {
            id: respond_id.clone(),
            node_type: "apiRespond".into(),
            name: "Respond to API".into(),
            position: botflow_workflow_model::Position { x: 360.0, y: 120.0 },
            parameters: json!({
                "statusCode": 200,
                "responseData": "firstEntryJson",
                "responseSchema": { "type": "object", "properties": {}, "required": [] },
            }),
            settings: None,
        },
    ];
    let edges = vec![botflow_workflow_model::WorkflowEdge {
        id: format!("e_{}", short_id(8)),
        source: trigger_id,
        target: respond_id,
        source_handle: None,
        target_handle: None,
    }];
    let wf = botflow_workflow_model::WorkflowDoc {
        id: None,
        name,
        description: Some(String::new()),
        enabled: true,
        version: 1,
        schema_version: botflow_workflow_model::WORKFLOW_SCHEMA_VERSION,
        nodes: nodes.clone(),
        edges,
        tags: Some(vec![tag]),
        time_saved_minutes: None,
        kind: Some("apiEndpoint".into()),
        api_project_id: Some(id.clone()),
        created_by: Some(user.sub.clone()),
        updated_by: Some(user.sub.clone()),
        owner_id: None,
        visibility: None,
        created_at: None,
        updated_at: None,
    };
    match state.app.create_workflow(wf).await {
        Ok(created) => {
            let wid = created.id.clone().unwrap_or_default();
            let _ = botflow_storage::sync_api_endpoints_from_workflow(
                &state.app.db,
                &wid,
                Some(&id),
                Some("apiEndpoint"),
                &created.nodes,
            )
            .await;
            Json(json!({ "workflow": created })).into_response()
        }
        Err(e) => error_response(e),
    }
}
