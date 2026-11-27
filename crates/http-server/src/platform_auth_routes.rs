//! Platform auth HTTP API: companies, roles, permissions, linked identities, cutover.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::AuthUser;
use crate::routes::{error_response, HttpState};

pub fn platform_auth_router() -> Router<HttpState> {
    Router::new()
        .route(
            "/api/companies",
            get(list_companies).post(create_company),
        )
        .route(
            "/api/companies/{id}",
            get(get_company)
                .patch(patch_company)
                .delete(delete_company),
        )
        .route("/api/roles", get(list_roles).post(create_role))
        .route(
            "/api/roles/{id}",
            get(get_role).patch(patch_role).delete(delete_role),
        )
        .route("/api/permissions", get(list_permissions))
        .route(
            "/api/linked-identities",
            get(list_linked_identities).post(create_linked_identity),
        )
        .route(
            "/api/linked-identities/{id}",
            axum::routing::patch(patch_linked_identity).delete(delete_linked_identity),
        )
        .route("/api/platform/cutover", get(cutover_status))
        .route("/api/bff/contract", get(bff_contract))
        .route("/api/auth/token", post(issue_user_token))
        .route("/api/auth/context", get(auth_context_me))
}

async fn list_companies(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_companies(&state.app.db).await {
        Ok(companies) => Json(json!({ "companies": companies })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_company(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::get_company(&state.app.db, &id).await {
        Ok(Some(c)) => Json(json!({ "company": c })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCompanyBody {
    name: String,
    #[serde(default)]
    external_ref: Option<String>,
}

async fn create_company(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<CreateCompanyBody>,
) -> Response {
    match botflow_storage::create_company(
        &state.app.db,
        &body.name,
        body.external_ref.as_deref(),
    )
    .await
    {
        Ok(c) => (StatusCode::CREATED, Json(json!({ "company": c }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchCompanyBody {
    name: Option<String>,
    status: Option<String>,
    external_ref: Option<String>,
}

async fn patch_company(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<PatchCompanyBody>,
) -> Response {
    match botflow_storage::update_company(
        &state.app.db,
        &id,
        body.name.as_deref(),
        body.status.as_deref(),
        body.external_ref.as_deref(),
    )
    .await
    {
        Ok(Some(c)) => Json(json!({ "company": c })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_company(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::delete_company(&state.app.db, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_roles(State(state): State<HttpState>, _user: AuthUser) -> Response {
    match botflow_storage::list_roles(&state.app.db).await {
        Ok(roles) => Json(json!({ "roles": roles })).into_response(),
        Err(e) => error_response(e),
    }
}

async fn get_role(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::get_role(&state.app.db, &id).await {
        Ok(Some(r)) => Json(json!({ "role": r })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRoleBody {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    permissions: Vec<String>,
}

async fn create_role(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<CreateRoleBody>,
) -> Response {
    match botflow_storage::create_role(
        &state.app.db,
        &body.id,
        &body.name,
        body.description.as_deref().unwrap_or(""),
        &body.permissions,
    )
    .await
    {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "role": r }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchRoleBody {
    name: Option<String>,
    description: Option<String>,
    permissions: Option<Vec<String>>,
}

async fn patch_role(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<PatchRoleBody>,
) -> Response {
    match botflow_storage::update_role(
        &state.app.db,
        &id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.permissions.as_deref(),
    )
    .await
    {
        Ok(Some(r)) => Json(json!({ "role": r })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_role(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::delete_role(&state.app.db, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_permissions(_user: AuthUser) -> Response {
    Json(json!({ "permissions": botflow_storage::list_permissions_catalog().await })).into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkedQuery {
    #[serde(default)]
    principal_id: Option<String>,
}

async fn list_linked_identities(
    State(state): State<HttpState>,
    _user: AuthUser,
    Query(q): Query<LinkedQuery>,
) -> Response {
    match botflow_storage::list_linked_identities(&state.app.db, q.principal_id.as_deref()).await {
        Ok(items) => Json(json!({ "linkedIdentities": items })).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLinkedBody {
    principal_id: String,
    provider: String,
    external_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    is_default: bool,
    #[serde(default)]
    is_primary: bool,
    #[serde(default)]
    metadata: Option<Value>,
}

async fn create_linked_identity(
    State(state): State<HttpState>,
    _user: AuthUser,
    Json(body): Json<CreateLinkedBody>,
) -> Response {
    match botflow_storage::create_linked_identity(
        &state.app.db,
        &body.principal_id,
        &body.provider,
        &body.external_id,
        body.label.as_deref(),
        body.is_default,
        body.is_primary,
        body.metadata,
    )
    .await
    {
        Ok(item) => (StatusCode::CREATED, Json(json!({ "linkedIdentity": item }))).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchLinkedBody {
    label: Option<String>,
    is_default: Option<bool>,
    is_primary: Option<bool>,
}

async fn patch_linked_identity(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<PatchLinkedBody>,
) -> Response {
    match botflow_storage::update_linked_identity(
        &state.app.db,
        &id,
        body.label.as_deref(),
        body.is_default,
        body.is_primary,
    )
    .await
    {
        Ok(Some(item)) => Json(json!({ "linkedIdentity": item })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn delete_linked_identity(
    State(state): State<HttpState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Response {
    match botflow_storage::delete_linked_identity(&state.app.db, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => error_response(e),
    }
}

async fn cutover_status(_user: AuthUser) -> Response {
    Json(botflow_storage::cutover_checklist()).into_response()
}

/// Contract for dumb client shells: one BotFlow call per screen action.
async fn bff_contract(_user: AuthUser) -> Response {
    Json(json!({
        "version": 1,
        "principle": "Shells render JSON only — no business rules in the client.",
        "auth": {
            "login": "POST /api/v1/auth/login or POST /api/auth/token",
            "header": "Authorization: Bearer <token>",
            "context": "GET /api/auth/context"
        },
        "screens": {
            "pattern": "GET|POST /apis/{slug}/{screenPath}",
            "authMode": "jwt",
            "response": {
                "viewModel": "object ready for binding",
                "actions": ["optional list of allowed next actions"],
                "auth": "echo of botflow auth context when useful"
            }
        },
        "performance": [
            "One round-trip per screen action",
            "Batch reference data in the flow",
            "Cache stable lookups in workflow nodes / credentials"
        ],
        "integrations": "Use nodes (cmCom, dynamics, mssql, httpRequest) inside flows — never call vendors from the shell."
    }))
    .into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueTokenBody {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    /// When already authenticated as admin, mint a token for this user id.
    #[serde(default)]
    principal_id: Option<String>,
}

async fn issue_user_token(
    State(state): State<HttpState>,
    user: AuthUser,
    Json(body): Json<IssueTokenBody>,
) -> Response {
    // Password login against user store (platform issuer).
    if let (Some(username), Some(password)) = (body.username.as_deref(), body.password.as_deref()) {
        let username = username.trim();
        match botflow_storage::find_user_by_identity(&state.app.db, username).await {
            Ok(Some(u)) => {
                let stored = u
                    .get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let expected = botflow_storage::hash_local_password(password);
                // find_user strips password — need raw lookup for verify.
                // Re-fetch via identity is stripped; compare using a dedicated path:
                let _ = stored;
                let _ = expected;
                // Password is stripped from public JSON. Use env-admin fallback + role from doc.
                let role = u.get("role").and_then(|v| v.as_str()).unwrap_or("shell_user");
                let sub = u
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(username)
                    .to_string();
                // Verify password via Mongo direct for security.
                match verify_user_password(&state, username, password).await {
                    Ok(true) => match state.jwt.issue(&sub, role) {
                        Ok(token) => {
                            let ctx = botflow_storage::build_auth_context(&state.app.db, &sub, role)
                                .await
                                .unwrap_or(json!({ "sub": sub, "role": role }));
                            return Json(json!({ "token": token, "role": role, "auth": ctx }))
                                .into_response();
                        }
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({ "error": e.to_string() })),
                            )
                                .into_response();
                        }
                    },
                    Ok(false) => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({ "error": "invalid credentials" })),
                        )
                            .into_response();
                    }
                    Err(e) => return error_response(e),
                }
            }
            Ok(None) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "invalid credentials" })),
                )
                    .into_response();
            }
            Err(e) => return error_response(e),
        }
    }

    // Admin mint for a principal.
    if user.role == "admin" || state.auth_disabled {
        if let Some(pid) = body.principal_id.as_deref().map(str::trim).filter(|s| !s.is_empty())
        {
            match botflow_storage::find_user_by_id(&state.app.db, pid).await {
                Ok(Some(u)) => {
                    let role = u.get("role").and_then(|v| v.as_str()).unwrap_or("shell_user");
                    match state.jwt.issue(pid, role) {
                        Ok(token) => {
                            let ctx = botflow_storage::build_auth_context(&state.app.db, pid, role)
                                .await
                                .unwrap_or(json!({ "sub": pid, "role": role }));
                            return Json(json!({ "token": token, "role": role, "auth": ctx }))
                                .into_response();
                        }
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({ "error": e.to_string() })),
                            )
                                .into_response();
                        }
                    }
                }
                Ok(None) => {
                    return (StatusCode::NOT_FOUND, Json(json!({ "error": "user not found" })))
                        .into_response();
                }
                Err(e) => return error_response(e),
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "username/password or principalId required" })),
    )
        .into_response()
}

async fn verify_user_password(
    state: &HttpState,
    identity: &str,
    password: &str,
) -> anyhow::Result<bool> {
    // Password is stripped from public API; verify via update path using hash compare
    // against a temporary re-read. We store local-sha256 hashes.
    use mongodb::bson::doc;
    let db = state.app.db.client.default_database().unwrap_or_else(|| {
        state.app.db.client.database("botflow")
    });
    let coll = db.collection::<mongodb::bson::Document>("user");
    let filter = doc! {
        "$or": [
            { "id": identity },
            { "username": identity },
            { "email": identity },
        ]
    };
    let Some(doc) = coll.find_one(filter).await? else {
        return Ok(false);
    };
    let stored = doc.get_str("password").unwrap_or("");
    let expected = botflow_storage::hash_local_password(password);
    Ok(stored == expected)
}

async fn auth_context_me(State(state): State<HttpState>, user: AuthUser) -> Response {
    match botflow_storage::build_auth_context(&state.app.db, &user.sub, &user.role).await {
        Ok(ctx) => Json(json!({ "auth": ctx })).into_response(),
        Err(e) => error_response(e),
    }
}
