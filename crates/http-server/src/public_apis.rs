//! Public `/apis/{slug}` surface: OpenAPI, Swagger UI, and endpoint dispatch.

use crate::routes::HttpState;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use botflow_security::verify_hs256_jwt;
use botflow_workflow_model::RunStatus;
use serde_json::{json, Map, Value};
use std::{collections::HashMap, time::Duration};
use tracing::error;

pub fn router() -> Router<HttpState> {
    Router::new()
        .route("/apis/{slug}/openapi.json", get(openapi_json))
        .route("/apis/{slug}/swagger", get(swagger_ui))
        .route("/apis/{slug}/{*rest}", any(dispatch))
}

fn project_id(project: &Value) -> String {
    project
        .get("_id")
        .or_else(|| project.get("id"))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string().trim_matches('"').to_string(),
        })
        .unwrap_or_default()
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_matches('/').to_string()
}

fn public_base_url() -> String {
    std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| {
        let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = std::env::var("PORT").unwrap_or_else(|_| "8788".into());
        format!("http://{host}:{port}")
    })
}

fn params_from_schema(schema: Option<&Value>, location: &str) -> Vec<Value> {
    let Some(schema) = schema.filter(|s| s.is_object()) else {
        return vec![];
    };
    let props = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();
    let required: std::collections::HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    props
        .into_iter()
        .map(|(name, prop)| {
            json!({
                "name": name,
                "in": location,
                "required": required.contains(&name),
                "schema": prop,
                "description": prop.get("description").and_then(|d| d.as_str()).unwrap_or(""),
            })
        })
        .collect()
}

fn path_params_from_template(path: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        if let Some(name) = part.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            out.push(json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": "string" },
            }));
            continue;
        }
        // Mixed tokens e.g. {referenceType}s
        if let Some(open) = part.find('{') {
            if let Some(close) = part.find('}') {
                if open < close {
                    let name = &part[open + 1..close];
                    out.push(json!({
                        "name": name,
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" },
                    }));
                }
            }
        }
    }
    out
}

pub fn build_openapi_document(project: &Value, endpoints: &[Value]) -> Value {
    let mut paths = Map::new();
    let auth = project.get("auth").cloned().unwrap_or(json!({}));
    let use_jwt = auth.get("mode").and_then(|m| m.as_str()) == Some("jwt");
    let use_api_key = endpoints
        .iter()
        .any(|e| e.get("auth").and_then(|a| a.as_str()) == Some("apiKey"));
    let scheme_name = auth
        .get("schemeName")
        .and_then(|s| s.as_str())
        .unwrap_or("Bearer");

    for ep in endpoints {
        let path = normalize_path(&json_str(ep, "path"));
        if path.is_empty() {
            continue;
        }
        let path_key = format!("/{path}");
        let method = json_str(ep, "method").to_lowercase();
        if method.is_empty() {
            continue;
        }

        let mut parameters = path_params_from_template(&path);
        parameters.extend(params_from_schema(ep.get("requestQuerySchema"), "query"));

        let response_status = ep
            .get("responseStatus")
            .and_then(|v| v.as_u64())
            .unwrap_or(200)
            .to_string();
        let mut responses = Map::new();
        let mut success = json!({ "description": "Success" });
        if let Some(schema) = ep.get("responseSchema") {
            if !schema.is_null() {
                success["content"] = json!({
                    "application/json": { "schema": schema }
                });
            }
        }
        responses.insert(response_status, success);
        responses.insert("401".into(), json!({ "description": "Unauthorized" }));
        responses.insert(
            "500".into(),
            json!({
                "description": "Error",
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "object",
                            "properties": { "message": { "type": "string" } }
                        }
                    }
                }
            }),
        );

        let mut op = json!({
            "operationId": ep.get("operationId").and_then(|v| v.as_str()).unwrap_or(&format!("{}_{}", method, path)),
            "summary": ep.get("summary").and_then(|v| v.as_str()).unwrap_or(&path),
            "tags": [ep.get("tag").and_then(|v| v.as_str()).unwrap_or("default")],
            "parameters": parameters,
            "responses": responses,
        });

        if matches!(method.as_str(), "post" | "put" | "patch") {
            if let Some(body_schema) = ep.get("requestBodySchema") {
                if !body_schema.is_null() {
                    op["requestBody"] = json!({
                        "required": true,
                        "content": {
                            "application/json": { "schema": body_schema }
                        }
                    });
                }
            }
        }

        let ep_auth = ep.get("auth").and_then(|a| a.as_str()).unwrap_or("none");
        op["security"] = match ep_auth {
            "apiKey" => json!([{ "ApiKey": [] }]),
            "jwt" | "headerBearer" => json!([{ scheme_name: [] }]),
            _ => json!([]),
        };

        let entry = paths.entry(path_key).or_insert_with(|| json!({}));
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(method, op);
        }
    }

    let mut schemas = project
        .get("schemas")
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_default();
    schemas.remove("__seedVersion");

    let mut components = json!({ "schemas": schemas });
    if use_jwt || use_api_key {
        let mut schemes = Map::new();
        if use_jwt {
            schemes.insert(
                scheme_name.into(),
                json!({
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT",
                }),
            );
        }
        if use_api_key {
            schemes.insert(
                "ApiKey".into(),
                json!({
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-Api-Key",
                    "description": "API key required (X-Api-Key header).",
                }),
            );
        }
        components["securitySchemes"] = Value::Object(schemes);
    }

    let slug = json_str(project, "slug");
    let mut version = json_str(project, "version");
    if version.is_empty() {
        version = "v1".into();
    }
    let security = if use_jwt {
        json!([{ scheme_name: [] }])
    } else {
        json!([])
    };
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": json_str(project, "name"),
            "description": json_str(project, "description"),
            "version": version,
        },
        "servers": [{
            "url": format!("{}/apis/{}", public_base_url(), slug),
            "description": "BotFlow API Integration",
        }],
        "paths": paths,
        "components": components,
        "security": security,
    })
}

fn swagger_ui_html(spec_url: &str, title: &str) -> String {
    let safe_title = title.replace('<', "&lt;");
    let spec = serde_json::to_string(spec_url).unwrap_or_else(|_| "\"\"".into());
    // Use r## so embedded "#id" selectors do not terminate the raw string.
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>{safe_title} — Swagger</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui.css" />
  <style>body{{margin:0}} .topbar{{display:none}} #bf-auth-hint{{padding:10px 16px;font:14px/1.4 system-ui,sans-serif;background:#fff8e6;border-bottom:1px solid #f0e0a0;color:#5c4b00}}</style>
</head>
<body>
  <div id="bf-auth-hint">Authorize with JWT (Bearer) or X-Api-Key. The OpenAPI document is filtered to endpoints your credentials may call.</div>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js"></script>
  <script>
    (async function () {{
      const specUrl = {spec};
      const params = new URLSearchParams(location.search);
      const token = params.get("token") || "";
      const apiKey = params.get("api_key") || params.get("apiKey") || "";
      const headers = {{}};
      if (token) headers["Authorization"] = "Bearer " + token;
      if (apiKey) headers["X-Api-Key"] = apiKey;
      let specDoc;
      try {{
        const res = await fetch(specUrl, {{ headers }});
        if (!res.ok) {{
          document.getElementById("swagger-ui").textContent =
            "Failed to load OpenAPI (" + res.status + "). Provide ?token= or ?api_key= with credentials that can access this API.";
          return;
        }}
        specDoc = await res.json();
      }} catch (e) {{
        document.getElementById("swagger-ui").textContent = "Failed to load OpenAPI: " + e;
        return;
      }}
      const schemes = (specDoc.components && specDoc.components.securitySchemes) || {{}};
      const bearerName = Object.keys(schemes).find(function (k) {{
        const s = schemes[k];
        return s && s.type === "http" && String(s.scheme || "").toLowerCase() === "bearer";
      }}) || "Bearer";
      window.ui = SwaggerUIBundle({{
        spec: specDoc,
        dom_id: "#swagger-ui",
        deepLinking: true,
        persistAuthorization: true,
        presets: [SwaggerUIBundle.presets.apis],
        layout: "BaseLayout",
        requestInterceptor: function (req) {{
          if (token && !req.headers.Authorization && !req.headers.authorization) {{
            req.headers.Authorization = "Bearer " + token;
          }}
          if (apiKey && !req.headers["X-Api-Key"] && !req.headers["x-api-key"]) {{
            req.headers["X-Api-Key"] = apiKey;
          }}
          return req;
        }},
        onComplete: function () {{
          try {{
            if (token) window.ui.preauthorizeApiKey(bearerName, token);
            if (apiKey) window.ui.preauthorizeApiKey("ApiKey", apiKey);
          }} catch (_) {{}}
        }}
      }});
    }})();
  </script>
</body>
</html>"##
    )
}

/// Match OpenAPI-style templates e.g. Opportunity/{{id}}/Won
fn match_path_template(template: &str, actual: &str) -> Option<HashMap<String, String>> {
    let t_parts: Vec<&str> = template.split('/').filter(|p| !p.is_empty()).collect();
    let a_parts: Vec<&str> = actual.split('/').filter(|p| !p.is_empty()).collect();
    if t_parts.len() != a_parts.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (t, a) in t_parts.into_iter().zip(a_parts) {
        if let Some(name) = t.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            params.insert(name.to_string(), a.to_string());
            continue;
        }
        if let Some(open) = t.find('{') {
            if let Some(close) = t.find('}') {
                if open == 0 && close + 1 < t.len() {
                    let name = &t[1..close];
                    let suffix = &t[close + 1..];
                    if a.ends_with(suffix) && a.len() > suffix.len() {
                        params.insert(
                            name.to_string(),
                            a[..a.len() - suffix.len()].to_string(),
                        );
                        continue;
                    }
                    return None;
                }
            }
        }
        if t != a {
            return None;
        }
    }
    Some(params)
}

struct ResolvedEndpoint {
    project: Value,
    endpoint: Value,
    workflow_id: String,
    path_params: HashMap<String, String>,
}

async fn resolve_endpoint(
    state: &HttpState,
    slug: &str,
    method: &str,
    rest_path: &str,
) -> Option<ResolvedEndpoint> {
    let project = botflow_storage::find_api_project_by_slug(&state.app.db, slug)
        .await
        .ok()
        .flatten()?;
    if project.get("enabled").and_then(|v| v.as_bool()) == Some(false) {
        return None;
    }
    let pid = project_id(&project);
    let path = normalize_path(rest_path);
    let method_upper = method.to_uppercase();
    let endpoints = botflow_storage::list_api_endpoints_for_project(&state.app.db, &pid)
        .await
        .ok()?;

    let mut path_params = HashMap::new();
    let mut endpoint = endpoints.iter().find(|ep| {
        json_str(ep, "method").eq_ignore_ascii_case(&method_upper)
            && normalize_path(&json_str(ep, "path")) == path
    });

    if endpoint.is_none() {
        for cand in &endpoints {
            if !json_str(cand, "method").eq_ignore_ascii_case(&method_upper) {
                continue;
            }
            let template = normalize_path(&json_str(cand, "path"));
            if !template.contains('{') {
                continue;
            }
            if let Some(matched) = match_path_template(&template, &path) {
                path_params = matched;
                endpoint = Some(cand);
                break;
            }
        }
    }

    let endpoint = endpoint?.clone();
    let workflow_id = json_str(&endpoint, "workflowId");
    if workflow_id.is_empty() {
        return None;
    }
    let wf = state.app.get_workflow(&workflow_id).await.ok().flatten()?;
    if !wf.enabled {
        return None;
    }

    Some(ResolvedEndpoint {
        project,
        endpoint,
        workflow_id,
        path_params,
    })
}

fn extract_respond(run_output: Option<&Value>, node_states: &[botflow_workflow_model::NodeRunState]) -> (u16, Value, Option<Map<String, Value>>) {
    let candidate = run_output
        .and_then(unwrap_respond_value)
        .or_else(|| {
            node_states
                .iter()
                .rev()
                .find_map(|n| n.output.as_ref().and_then(unwrap_respond_value))
        });

    if let Some(obj) = candidate {
        let status = obj
            .get("statusCode")
            .and_then(|v| v.as_u64())
            .unwrap_or(200) as u16;
        let body = obj.get("body").cloned().unwrap_or(Value::Null);
        if let Some(wrapped) = body.as_object() {
            if wrapped.contains_key("headers") && wrapped.contains_key("body") {
                let headers = wrapped
                    .get("headers")
                    .and_then(|h| h.as_object())
                    .cloned();
                let inner = wrapped.get("body").cloned().unwrap_or(Value::Null);
                return (status, inner, headers);
            }
        }
        return (status, body, None);
    }

    (
        200,
        run_output.cloned().unwrap_or(Value::Null),
        None,
    )
}

fn unwrap_respond_value(v: &Value) -> Option<Value> {
    match v {
        Value::Array(items) => items.iter().rev().find_map(unwrap_respond_value),
        Value::Object(map) => {
            if map.contains_key("statusCode") && map.contains_key("body") {
                Some(Value::Object(map.clone()))
            } else if map.get("respond").and_then(|r| r.as_bool()) == Some(true) {
                Some(Value::Object(map.clone()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            let v = v.trim();
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
}

fn api_key_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.iter().find_map(|(k, v)| {
        if k.as_str().eq_ignore_ascii_case("x-api-key") {
            v.to_str().ok().map(|s| s.trim().to_string())
        } else {
            None
        }
    })
    .filter(|s| !s.is_empty())
}

fn endpoint_doc_id(endpoint: &Value) -> String {
    json_str(endpoint, "id")
}

fn forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({ "message": "Forbidden" })),
    )
        .into_response()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "message": "Unauthorized" })),
    )
        .into_response()
}

/// Enforce `api.invoke` + effective `apiAccess` for a JWT principal context.
fn enforce_jwt_api_access(
    ctx: &Value,
    project: &Value,
    endpoint: &Value,
    method: &str,
    path: &str,
) -> Result<(), Response> {
    let perms: Vec<String> = ctx
        .get("permissions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if !botflow_storage::permissions_include_api_invoke(&perms) {
        return Err(forbidden());
    }
    let access = ctx.get("apiAccess").cloned().unwrap_or_else(|| json!({}));
    let pid = project_id(project);
    let slug = json_str(project, "slug");
    let eid = endpoint_doc_id(endpoint);
    if !botflow_storage::api_access_allows(
        &access,
        &pid,
        &slug,
        method,
        path,
        if eid.is_empty() { None } else { Some(eid.as_str()) },
    ) {
        return Err(forbidden());
    }
    Ok(())
}

/// Caller access used to filter OpenAPI/Swagger to allowed operations.
struct DocsAccess {
    /// Access document (`apiAccess` or API key metadata). `None` = anonymous.
    access: Option<Value>,
    /// When true, caller must have `api.invoke` (JWT users). API keys skip this.
    require_api_invoke: bool,
    permissions: Vec<String>,
}

fn jwt_permissions(ctx: &Value) -> Vec<String> {
    ctx.get("permissions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve JWT or API key from docs request headers / query. Invalid creds → 401.
async fn resolve_docs_access(
    state: &HttpState,
    headers: &HeaderMap,
    query_token: Option<&str>,
    query_api_key: Option<&str>,
) -> Result<DocsAccess, Response> {
    let bearer = bearer_from_headers(headers)
        .or_else(|| {
            query_token
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        });
    let api_key = api_key_from_headers(headers).or_else(|| {
        query_api_key
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });

    if let Some(token) = bearer {
        if let Ok(claims) = state.jwt.verify(&token) {
            let ctx = botflow_storage::build_auth_context(
                &state.app.db,
                &claims.sub,
                &claims.role,
            )
            .await
            .unwrap_or_else(|_| {
                json!({
                    "sub": claims.sub,
                    "role": claims.role,
                    "permissions": [],
                    "apiAccess": {}
                })
            });
            return Ok(DocsAccess {
                access: Some(
                    ctx.get("apiAccess")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                ),
                require_api_invoke: true,
                permissions: jwt_permissions(&ctx),
            });
        }
        // Project-signed JWTs still need a BotFlow role context when possible.
        // For docs filtering, only instance JWTs are accepted as Bearer identity.
        return Err(unauthorized());
    }

    if let Some(key) = api_key {
        if let Ok(Some(verified)) =
            botflow_storage::verify_instance_api_key(&state.app.db, &key).await
        {
            let meta = verified
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| json!({}));
            return Ok(DocsAccess {
                access: Some(meta),
                require_api_invoke: false,
                permissions: vec![],
            });
        }
        return Err(unauthorized());
    }

    Ok(DocsAccess {
        access: None,
        require_api_invoke: false,
        permissions: vec![],
    })
}

/// Filter endpoints for OpenAPI: anonymous sees only `auth: none`; authenticated
/// callers see endpoints their access allows.
fn filter_endpoints_for_docs(
    project: &Value,
    endpoints: &[Value],
    docs: &DocsAccess,
) -> Vec<Value> {
    let pid = project_id(project);
    let slug = json_str(project, "slug");
    endpoints
        .iter()
        .filter(|ep| {
            let ep_auth = ep.get("auth").and_then(|a| a.as_str()).unwrap_or("none");
            if ep_auth == "none" || ep_auth.is_empty() {
                return true;
            }
            let Some(access) = docs.access.as_ref() else {
                return false;
            };
            if docs.require_api_invoke
                && !botflow_storage::permissions_include_api_invoke(&docs.permissions)
            {
                return false;
            }
            let method = json_str(ep, "method");
            let path = normalize_path(&json_str(ep, "path"));
            let eid = endpoint_doc_id(ep);
            botflow_storage::api_access_allows(
                access,
                &pid,
                &slug,
                &method,
                &path,
                if eid.is_empty() {
                    None
                } else {
                    Some(eid.as_str())
                },
            )
        })
        .cloned()
        .collect()
}

async fn authorize_endpoint(
    state: &HttpState,
    project: &Value,
    endpoint: &Value,
    method: &str,
    path: &str,
    headers: &HeaderMap,
) -> Result<Option<Value>, Response> {
    let ep_auth = endpoint.get("auth").and_then(|a| a.as_str()).unwrap_or("none");
    let eid = endpoint_doc_id(endpoint);
    let eid_opt = if eid.is_empty() {
        None
    } else {
        Some(eid.as_str())
    };
    match ep_auth {
        "none" | "" => Ok(None),
        "apiKey" => {
            let key = api_key_from_headers(headers).ok_or_else(unauthorized)?;
            let pid = project_id(project);
            let slug = json_str(project, "slug");
            if let Ok(Some(verified)) =
                botflow_storage::verify_instance_api_key(&state.app.db, &key).await
            {
                if botflow_storage::instance_api_key_allows(
                    &verified, &pid, &slug, method, path, eid_opt,
                ) {
                    return Ok(Some(botflow_storage::api_key_auth_context(&verified)));
                }
                return Err(forbidden());
            }
            Err(unauthorized())
        }
        "headerBearer" => {
            let token = bearer_from_headers(headers).ok_or_else(unauthorized)?;
            let expected = project
                .get("auth")
                .and_then(|a| a.get("jwtSecret"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if expected.is_empty() || token != expected {
                return Err(unauthorized());
            }
            Ok(Some(json!({ "bearer": true })))
        }
        "jwt" => {
            let token = bearer_from_headers(headers).ok_or_else(unauthorized)?;
            let secret = project
                .get("auth")
                .and_then(|a| a.get("jwtSecret"))
                .and_then(|s| s.as_str())
                .unwrap_or("botflow-dev-jwt-secret");
            match verify_hs256_jwt(&token, secret) {
                Ok(payload) => {
                    let sub = payload
                        .get("sub")
                        .or_else(|| payload.get("nameid"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let role = payload
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("shell_user");
                    let principal = if sub.is_empty() { "anonymous" } else { sub };
                    let ctx = botflow_storage::build_auth_context(
                        &state.app.db,
                        principal,
                        role,
                    )
                    .await
                    .unwrap_or_else(|_| {
                        json!({
                            "sub": principal,
                            "role": role,
                            "permissions": [],
                            "apiAccess": {}
                        })
                    });
                    enforce_jwt_api_access(&ctx, project, endpoint, method, path)?;
                    let mut merged = payload;
                    if let Some(obj) = merged.as_object_mut() {
                        obj.insert("botflow".into(), ctx);
                    }
                    Ok(Some(merged))
                }
                Err(_) => {
                    // Also accept studio-issued BotFlow JWTs (instance secret).
                    if let Ok(claims) = state.jwt.verify(&token) {
                        let ctx = botflow_storage::build_auth_context(
                            &state.app.db,
                            &claims.sub,
                            &claims.role,
                        )
                        .await
                        .unwrap_or_else(|_| {
                            json!({
                                "sub": claims.sub.clone(),
                                "role": claims.role.clone(),
                                "permissions": [],
                                "apiAccess": {}
                            })
                        });
                        enforce_jwt_api_access(&ctx, project, endpoint, method, path)?;
                        return Ok(Some(json!({
                            "sub": claims.sub,
                            "role": claims.role,
                            "botflow": ctx,
                        })));
                    }
                    Err(unauthorized())
                }
            }
        }
        other => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "message": format!("Unsupported auth mode: {other}") })),
        )
            .into_response()),
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct DocsQuery {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default, alias = "apiKey")]
    api_key_camel: Option<String>,
}

async fn openapi_json(
    State(state): State<HttpState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Query(query): Query<DocsQuery>,
) -> Response {
    let Some(project) = botflow_storage::find_api_project_by_slug(&state.app.db, &slug)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "API not found" })),
        )
            .into_response();
    };
    let api_key = query
        .api_key
        .as_deref()
        .or(query.api_key_camel.as_deref());
    let docs = match resolve_docs_access(
        &state,
        &headers,
        query.token.as_deref(),
        api_key,
    )
    .await
    {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    let pid = project_id(&project);
    let endpoints = botflow_storage::list_api_endpoints_for_project(&state.app.db, &pid)
        .await
        .unwrap_or_default();
    let filtered = filter_endpoints_for_docs(&project, &endpoints, &docs);
    Json(build_openapi_document(&project, &filtered)).into_response()
}

async fn swagger_ui(
    State(state): State<HttpState>,
    Path(slug): Path<String>,
) -> Response {
    let Some(project) = botflow_storage::find_api_project_by_slug(&state.app.db, &slug)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "API not found" })),
        )
            .into_response();
    };
    let title = json_str(&project, "name");
    let title = if title.is_empty() { slug.clone() } else { title };
    // Spec URL is same-origin; Swagger page forwards ?token= / ?api_key= and Authorize headers.
    let spec_url = format!("/apis/{slug}/openapi.json");
    Html(swagger_ui_html(&spec_url, &title)).into_response()
}

async fn dispatch(
    State(state): State<HttpState>,
    Path((slug, rest)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let rest = normalize_path(&rest);
    if rest == "openapi.json" || rest == "swagger" {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response();
    }

    let Some(resolved) = resolve_endpoint(&state, &slug, method.as_str(), &rest).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "message": format!("No endpoint {} /{} on API {}", method, rest, slug)
            })),
        )
            .into_response();
    };

    let jwt = match authorize_endpoint(
        &state,
        &resolved.project,
        &resolved.endpoint,
        method.as_str(),
        &rest,
        &headers,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let mut query_obj = Map::new();
    for (k, v) in &query {
        query_obj.insert(k.clone(), Value::String(v.clone()));
    }
    for (k, v) in &resolved.path_params {
        query_obj.insert(k.clone(), Value::String(v.clone()));
    }

    let body_json = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice::<Value>(&body).unwrap_or_else(|_| {
            Value::String(String::from_utf8_lossy(&body).into_owned())
        })
    };

    let mut header_map = Map::new();
    for (k, v) in headers.iter() {
        if let Ok(s) = v.to_str() {
            header_map.insert(k.as_str().to_string(), Value::String(s.to_string()));
        }
    }

    let input = json!({
        "kind": "api",
        "apiProjectId": project_id(&resolved.project),
        "apiSlug": json_str(&resolved.project, "slug"),
        "method": method.as_str(),
        "path": rest,
        "headers": header_map,
        "query": query_obj,
        "params": resolved.path_params,
        "body": body_json,
        "jwt": jwt,
    });

    match state
        .app
        .run_workflow_and_wait(&resolved.workflow_id, input, Duration::from_secs(120))
        .await
    {
        Ok(run) => {
            if run.status == RunStatus::Failed {
                let message = run
                    .error
                    .clone()
                    .unwrap_or_else(|| "An unexpected error occurred.".into());
                error!(
                    slug = %slug,
                    method = %method,
                    path = %rest,
                    run_id = ?run.id,
                    error = %message,
                    "API endpoint run failed"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": message })),
                )
                    .into_response();
            }

            let (status, body, extra_headers) =
                extract_respond(run.output.as_ref(), &run.node_states);
            let mut response = (StatusCode::from_u16(status).unwrap_or(StatusCode::OK), Json(body))
                .into_response();
            if let Some(hdrs) = extra_headers {
                for (k, v) in hdrs {
                    if let (Ok(name), Some(val)) = (
                        HeaderName::from_bytes(k.as_bytes()),
                        v.as_str().and_then(|s| HeaderValue::from_str(s).ok()),
                    ) {
                        response.headers_mut().insert(name, val);
                    }
                }
            }
            response
        }
        Err(e) => {
            error!(
                slug = %slug,
                method = %method,
                path = %rest,
                error = %e,
                "API endpoint handler error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": e.to_string() })),
            )
                .into_response()
        }
    }
}
