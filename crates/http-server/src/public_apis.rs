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
            "description": "DataFlow API project",
        }],
        "paths": paths,
        "components": components,
        "security": security,
    })
}

fn swagger_ui_html(spec_url: &str, title: &str) -> String {
    let safe_title = title.replace('<', "&lt;");
    let spec = serde_json::to_string(spec_url).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>{safe_title} — Swagger</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui.css" />
  <style>body{{margin:0}} .topbar{{display:none}}</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({{
      url: {spec},
      dom_id: '#swagger-ui',
      deepLinking: true,
      presets: [SwaggerUIBundle.presets.apis],
      layout: "BaseLayout"
    }});
  </script>
</body>
</html>"#
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

async fn authorize_endpoint(
    state: &HttpState,
    project: &Value,
    endpoint: &Value,
    method: &str,
    path: &str,
    headers: &HeaderMap,
) -> Result<Option<Value>, Response> {
    let ep_auth = endpoint.get("auth").and_then(|a| a.as_str()).unwrap_or("none");
    match ep_auth {
        "none" | "" => Ok(None),
        "apiKey" => {
            let key = api_key_from_headers(headers).ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "message": "Unauthorized" })),
                )
                    .into_response()
            })?;
            let pid = project_id(project);
            let slug = json_str(project, "slug");
            if let Ok(Some(verified)) =
                botflow_storage::verify_instance_api_key(&state.app.db, &key).await
            {
                if botflow_storage::instance_api_key_allows(&verified, &pid, &slug, method, path)
                {
                    return Ok(Some(json!({ "apiKeyId": verified.get("id") })));
                }
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({ "message": "Forbidden" })),
                )
                    .into_response());
            }
            let expected = project
                .get("auth")
                .and_then(|a| a.get("jwtSecret"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if !expected.is_empty() && key == expected {
                return Ok(Some(json!({ "apiKey": true })));
            }
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "message": "Unauthorized" })),
            )
                .into_response())
        }
        "headerBearer" => {
            let token = bearer_from_headers(headers).ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "message": "Unauthorized" })),
                )
                    .into_response()
            })?;
            let expected = project
                .get("auth")
                .and_then(|a| a.get("jwtSecret"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if expected.is_empty() || token != expected {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "message": "Unauthorized" })),
                )
                    .into_response());
            }
            Ok(Some(json!({ "bearer": true })))
        }
        "jwt" => {
            let token = bearer_from_headers(headers).ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "message": "Unauthorized" })),
                )
                    .into_response()
            })?;
            let secret = project
                .get("auth")
                .and_then(|a| a.get("jwtSecret"))
                .and_then(|s| s.as_str())
                .unwrap_or("botflow-dev-jwt-secret");
            match verify_hs256_jwt(&token, secret) {
                Ok(payload) => Ok(Some(payload)),
                Err(_) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "message": "Unauthorized" })),
                )
                    .into_response()),
            }
        }
        other => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "message": format!("Unsupported auth mode: {other}") })),
        )
            .into_response()),
    }
}

async fn openapi_json(
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
    let pid = project_id(&project);
    let endpoints = botflow_storage::list_api_endpoints_for_project(&state.app.db, &pid)
        .await
        .unwrap_or_default();
    Json(build_openapi_document(&project, &endpoints)).into_response()
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
