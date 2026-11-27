//! Schema-driven form shown before manual "Execute workflow".
//! Builds an apiEndpoint-shaped request item (`query` / `body`) from request schemas.

use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::{json, Map, Value};

use super::json_schema_sample::{
    parse_schema_value, schema_to_property_rows, SchemaPropertyRow,
};
use crate::components::icons::X;
use crate::components::ui::{
    Button, ButtonVariant, Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader,
    CardTitle, Checkbox, Dialog, DialogContent, Input, Label, Select, SelectOption, Textarea,
};

const AUTH_OPTIONS: &[(&str, &str)] = &[
    ("none", "None (AllowAnonymous)"),
    ("jwt", "JWT Bearer"),
    ("headerBearer", "Shared Bearer secret"),
    ("apiKey", "API Key (X-Api-Key)"),
];

const AUTH_HELP: &str =
    "JWT = signed-in user session; API Key = X-Api-Key (scoped keys / project secret); Shared Bearer = project secret.";

fn endpoint_auth_mode(parameters: &Value) -> String {
    parameters
        .get("auth")
        .and_then(|v| v.as_str())
        .unwrap_or("jwt")
        .to_string()
}

/// True when the execute dialog should open (schema fields and/or auth credential).
pub fn needs_execute_dialog(parameters: &Value, fields: &[FormField]) -> bool {
    if !fields.is_empty() {
        return true;
    }
    matches!(
        endpoint_auth_mode(parameters).as_str(),
        "jwt" | "headerBearer" | "apiKey"
    )
}

fn auth_credential_label(mode: &str) -> Option<&'static str> {
    match mode {
        "apiKey" => Some("API Key"),
        "headerBearer" => Some("Bearer secret"),
        _ => None,
    }
}

fn build_auth_headers(mode: &str, credential: &str) -> Map<String, Value> {
    let mut headers = Map::new();
    let cred = credential.trim();
    match mode {
        "apiKey" => {
            if !cred.is_empty() {
                headers.insert("X-Api-Key".into(), Value::String(cred.to_string()));
            }
        }
        "headerBearer" => {
            if !cred.is_empty() {
                headers.insert(
                    "Authorization".into(),
                    Value::String(format!("Bearer {cred}")),
                );
            }
        }
        "jwt" => {
            if let Some(token) = crate::api::auth_token() {
                if !token.is_empty() {
                    headers.insert(
                        "Authorization".into(),
                        Value::String(format!("Bearer {token}")),
                    );
                }
            }
        }
        _ => {}
    }
    headers
}

#[derive(Clone, PartialEq)]
pub struct FormField {
    pub section: &'static str,
    pub row: SchemaPropertyRow,
}

fn rows_for_schema(schema: Option<&Value>) -> Vec<SchemaPropertyRow> {
    let Some(raw) = schema else {
        return Vec::new();
    };
    schema_to_property_rows(&parse_schema_value(raw))
}

fn html_input_type(prop_type: &str) -> &'static str {
    match prop_type {
        "integer" | "number" => "number",
        "date" => "date",
        "date-time" => "datetime-local",
        "boolean" => "checkbox",
        _ => "text",
    }
}

fn default_field_value(row: &SchemaPropertyRow) -> String {
    let ex = row.example.trim();
    if !ex.is_empty() {
        if row.prop_type == "date-time" {
            return datetime_local_from_iso(ex);
        }
        return ex.to_string();
    }
    match row.prop_type.as_str() {
        "boolean" => "false".into(),
        "integer" | "number" => String::new(),
        "date" => chrono::Local::now().format("%Y-%m-%d").to_string(),
        "date-time" => chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string(),
        "array" => "[]".into(),
        "object" => "{}".into(),
        _ => String::new(),
    }
}

fn datetime_local_from_iso(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 16 && s.as_bytes().get(10) == Some(&b'T') {
        // 2026-01-01T00:00:00.000Z → 2026-01-01T00:00
        return s.chars().take(16).collect();
    }
    if s.len() >= 10 && s.as_bytes().get(4) == Some(&b'-') {
        return format!("{}T00:00", &s[..10]);
    }
    s.to_string()
}

fn iso_from_datetime_local(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.len() == 16 && s.as_bytes().get(10) == Some(&b'T') {
        return format!("{s}:00.000Z");
    }
    if s.len() == 19 && s.as_bytes().get(10) == Some(&b'T') {
        return format!("{s}.000Z");
    }
    s.to_string()
}

fn coerce_field_value(prop_type: &str, raw: &str) -> Value {
    let trimmed = raw.trim();
    match prop_type {
        "integer" => trimmed
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or(Value::Null),
        "number" => trimmed
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "boolean" => {
            Value::Bool(matches!(
                trimmed.to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            ))
        }
        "date-time" => {
            let iso = iso_from_datetime_local(trimmed);
            if iso.is_empty() {
                Value::Null
            } else {
                Value::String(iso)
            }
        }
        "array" | "object" => {
            if trimmed.is_empty() {
                if prop_type == "array" {
                    json!([])
                } else {
                    json!({})
                }
            } else {
                serde_json::from_str(trimmed).unwrap_or_else(|_| Value::String(trimmed.to_string()))
            }
        }
        _ => {
            if trimmed.is_empty() {
                Value::Null
            } else {
                Value::String(trimmed.to_string())
            }
        }
    }
}

fn infer_prop_type(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.contains("datetime") || lower.contains("date_time") || lower.ends_with("datetime") {
        return "date-time".into();
    }
    if lower == "date" || lower.ends_with("date") || lower.starts_with("date") {
        return "date".into();
    }
    if lower.starts_with("is") || lower.starts_with("has") || lower.contains("enabled") {
        return "boolean".into();
    }
    if matches!(
        lower.as_str(),
        "page" | "pagesize" | "limit" | "offset" | "count" | "watchlistid"
    ) {
        return "integer".into();
    }
    "string".into()
}

/// Pull `$json.query.foo` / `$json.body.bar` names from node parameter strings.
fn referenced_request_keys(nodes: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut query = Vec::new();
    let mut body = Vec::new();
    let blob = serde_json::to_string(nodes).unwrap_or_default();
    for (section, out) in [("query", &mut query), ("body", &mut body)] {
        let needle = format!("$json.{section}.");
        let mut start = 0;
        while let Some(rel) = blob[start..].find(&needle) {
            let after_start = start + rel + needle.len();
            let after = &blob[after_start..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && !out.iter().any(|n| n == &name) {
                out.push(name);
            }
            start = after_start + 1;
            if start >= blob.len() {
                break;
            }
        }
    }
    (query, body)
}

/// Collect schema fields from the first `apiEndpoint` node in `nodes`.
/// Also adds query/body keys referenced by downstream expressions (e.g. `$json.query.date`).
pub fn api_endpoint_form_fields(nodes: &[Value]) -> (Option<Value>, Vec<FormField>) {
    let endpoint = nodes.iter().find(|n| {
        n.get("type")
            .or_else(|| n.get("nodeType"))
            .and_then(|v| v.as_str())
            == Some("apiEndpoint")
    });
    let Some(node) = endpoint else {
        return (None, Vec::new());
    };
    let params = node.get("parameters").cloned().unwrap_or(json!({}));
    let mut fields = Vec::new();
    let mut seen_query = std::collections::HashSet::new();
    let mut seen_body = std::collections::HashSet::new();

    for row in rows_for_schema(params.get("requestQuerySchema")) {
        if row.name.trim().is_empty() {
            continue;
        }
        seen_query.insert(row.name.clone());
        fields.push(FormField {
            section: "query",
            row,
        });
    }
    for row in rows_for_schema(params.get("requestBodySchema")) {
        if row.name.trim().is_empty() {
            continue;
        }
        seen_body.insert(row.name.clone());
        fields.push(FormField {
            section: "body",
            row,
        });
    }

    let (ref_query, ref_body) = referenced_request_keys(nodes);
    for name in ref_query {
        if seen_query.contains(&name) {
            continue;
        }
        let prop_type = infer_prop_type(&name);
        seen_query.insert(name.clone());
        fields.push(FormField {
            section: "query",
            row: SchemaPropertyRow {
                name,
                prop_type,
                example: String::new(),
                required: false,
            },
        });
    }
    for name in ref_body {
        if seen_body.contains(&name) {
            continue;
        }
        let prop_type = infer_prop_type(&name);
        seen_body.insert(name.clone());
        fields.push(FormField {
            section: "body",
            row: SchemaPropertyRow {
                name,
                prop_type,
                example: String::new(),
                required: false,
            },
        });
    }

    (Some(params), fields)
}

pub fn build_run_input(parameters: &Value, values: &HashMap<String, String>) -> Value {
    build_run_input_with_auth(
        parameters,
        values,
        &endpoint_auth_mode(parameters),
        values.get("auth\0credential").map(|s| s.as_str()).unwrap_or(""),
    )
}

pub fn build_run_input_with_auth(
    parameters: &Value,
    values: &HashMap<String, String>,
    auth_mode: &str,
    auth_credential: &str,
) -> Value {
    let method = parameters
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");
    let path = parameters
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut query = Map::new();
    let mut body = Map::new();

    for (key, raw) in values {
        if key.starts_with("auth\0") {
            continue;
        }
        let Some((section, name)) = key.split_once('\0') else {
            continue;
        };
        let prop_type = key_prop_type(parameters, section, name);
        let coerced = coerce_field_value(&prop_type, raw);
        if coerced.is_null() && raw.trim().is_empty() {
            continue;
        }
        match section {
            "query" => {
                query.insert(name.to_string(), coerced);
            }
            "body" => {
                body.insert(name.to_string(), coerced);
            }
            _ => {}
        }
    }

    let body_value = if body.is_empty() {
        if matches!(
            method.to_ascii_uppercase().as_str(),
            "POST" | "PUT" | "PATCH"
        ) {
            json!({})
        } else {
            Value::Null
        }
    } else {
        Value::Object(body)
    };

    json!({
        "kind": "manual",
        "method": method,
        "path": path,
        "headers": build_auth_headers(auth_mode, auth_credential),
        "query": query,
        "body": body_value,
        "auth": auth_mode,
    })
}

fn key_prop_type(parameters: &Value, section: &str, name: &str) -> String {
    let schema_key = if section == "body" {
        "requestBodySchema"
    } else {
        "requestQuerySchema"
    };
    let schema = parameters
        .get(schema_key)
        .map(parse_schema_value)
        .unwrap_or(json!({}));
    schema
        .get("properties")
        .and_then(|p| p.get(name))
        .map(|prop| {
            let raw_type = prop.get("type").and_then(|v| v.as_str()).unwrap_or("string");
            let format = prop.get("format").and_then(|v| v.as_str()).unwrap_or("");
            if raw_type == "string" {
                match format {
                    "date" => return "date".into(),
                    "date-time" => return "date-time".into(),
                    _ => {}
                }
            }
            raw_type.to_string()
        })
        .unwrap_or_else(|| "string".into())
}

fn field_key(section: &str, name: &str) -> String {
    format!("{section}\0{name}")
}

#[component]
pub fn ExecuteWorkflowDialog(
    parameters: Value,
    fields: Vec<FormField>,
    busy: bool,
    on_cancel: EventHandler<()>,
    on_execute: EventHandler<Value>,
) -> Element {
    let initial: HashMap<String, String> = fields
        .iter()
        .map(|f| {
            (
                field_key(f.section, &f.row.name),
                default_field_value(&f.row),
            )
        })
        .collect();
    let values = use_signal(|| initial);
    let mut auth_mode = use_signal(|| endpoint_auth_mode(&parameters));
    let mut auth_credential = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let has_query = fields.iter().any(|f| f.section == "query");
    let has_body = fields.iter().any(|f| f.section == "body");
    let method = parameters
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_string();
    let path = parameters
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let params_for_submit = parameters.clone();
    let fields_for_submit = fields.clone();
    let submit = move |_| {
        let map = values();
        for f in &fields_for_submit {
            if !f.row.required || f.row.prop_type == "boolean" {
                continue;
            }
            let key = field_key(f.section, &f.row.name);
            let v = map.get(&key).map(|s| s.trim()).unwrap_or("");
            if v.is_empty() {
                error.set(Some(format!("{} is required", f.row.name)));
                return;
            }
        }
        let mode = auth_mode();
        if auth_credential_label(&mode).is_some() && auth_credential().trim().is_empty() {
            error.set(Some(format!(
                "{} is required",
                auth_credential_label(&mode).unwrap_or("Credential")
            )));
            return;
        }
        error.set(None);
        on_execute.call(build_run_input_with_auth(
            &params_for_submit,
            &map,
            &mode,
            &auth_credential(),
        ));
    };

    rsx! {
        Dialog {
            on_close: move |_| {
                if !busy {
                    on_cancel.call(());
                }
            },
            DialogContent {
                class: Some(
                    "max-h-none max-w-lg gap-0 border-0 bg-transparent p-0 shadow-none".into(),
                ),
                show_close_button: false,
                on_close: move |_| {
                    if !busy {
                        on_cancel.call(());
                    }
                },
                Card { class: Some("w-full max-h-[min(85vh,720px)] overflow-hidden shadow-lg".into()),
                    CardHeader {
                        CardTitle { "Execute workflow" }
                        CardDescription {
                            if path.is_empty() {
                                "Provide request values, then run this workflow."
                            } else {
                                "{method} /{path}"
                            }
                        }
                        CardAction {
                            button {
                                r#type: "button",
                                class: "rounded-sm p-1 opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
                                "aria-label": "Close dialog",
                                disabled: busy,
                                onclick: move |_| on_cancel.call(()),
                                X { class: Some("size-4".into()) }
                            }
                        }
                    }

                    CardContent { class: Some("flex min-h-0 flex-col gap-4 overflow-y-auto".into()),
                        div { class: "flex max-h-[min(50vh,420px)] flex-col gap-4 overflow-y-auto pr-1",
                            div { class: "flex flex-col gap-2",
                                Label {
                                    r#for: Some("exec-auth-mode".into()),
                                    class: Some("text-muted-foreground".into()),
                                    "Authentication"
                                }
                                p { class: "text-xs text-muted-foreground", "{AUTH_HELP}" }
                                Select {
                                    id: Some("exec-auth-mode".into()),
                                    value: auth_mode(),
                                    onchange: move |e: FormEvent| {
                                        auth_mode.set(e.value());
                                        auth_credential.set(String::new());
                                    },
                                    for (value, label) in AUTH_OPTIONS {
                                        SelectOption {
                                            value: value.to_string(),
                                            selected: auth_mode() == *value,
                                            "{label}"
                                        }
                                    }
                                }
                                if let Some(cred_label) = auth_credential_label(&auth_mode()) {
                                    div { class: "flex flex-col gap-2",
                                        Label {
                                            r#for: Some("exec-auth-credential".into()),
                                            "{cred_label}"
                                            span { class: "text-destructive", " *" }
                                        }
                                        Input {
                                            id: Some("exec-auth-credential".into()),
                                            r#type: Some("password".into()),
                                            value: auth_credential(),
                                            required: true,
                                            placeholder: Some(format!("Paste {cred_label}")),
                                            oninput: move |e: FormEvent| {
                                                auth_credential.set(e.value());
                                            },
                                        }
                                    }
                                } else {
                                    p { class: "text-xs text-muted-foreground",
                                        "Uses your signed-in session — no key to enter."
                                    }
                                }
                            }
                            if has_query {
                                FieldSection {
                                    title: "Query parameters".to_string(),
                                    section: "query",
                                    fields: fields.iter().filter(|f| f.section == "query").cloned().collect(),
                                    values,
                                }
                            }
                            if has_body {
                                FieldSection {
                                    title: "Body".to_string(),
                                    section: "body",
                                    fields: fields.iter().filter(|f| f.section == "body").cloned().collect(),
                                    values,
                                }
                            }
                            if !has_query && !has_body {
                                p { class: "text-sm text-muted-foreground",
                                    "No request schema fields. Run with an empty request."
                                }
                            }
                        }

                        if let Some(msg) = error() {
                            p { class: "text-sm text-destructive", "{msg}" }
                        }
                    }

                    CardFooter {
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: busy,
                            onclick: move |_| on_cancel.call(()),
                            "Cancel"
                        }
                        Button {
                            disabled: busy,
                            onclick: submit,
                            if busy { "Executing…" } else { "Execute workflow" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FieldSection(
    title: String,
    section: &'static str,
    fields: Vec<FormField>,
    mut values: Signal<HashMap<String, String>>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-3 border-t border-border/60 pt-3",
            h4 { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "{title}" }
            for field in fields.into_iter() {
                {
                    let name = field.row.name.clone();
                    let prop_type = field.row.prop_type.clone();
                    let required = field.row.required;
                    let key = field_key(section, &name);
                    let id = format!("exec-{section}-{name}");
                    let input_ty = html_input_type(&prop_type);
                    let current = values().get(&key).cloned().unwrap_or_default();
                    let is_bool = prop_type == "boolean";
                    let is_json = matches!(prop_type.as_str(), "array" | "object");
                    let checked = matches!(
                        current.to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes" | "on"
                    );

                    rsx! {
                        div { class: "flex flex-col gap-2",
                            if is_bool {
                                div { class: "flex items-center gap-2",
                                    Checkbox {
                                        checked,
                                        aria_label: name.clone(),
                                        on_checked_change: move |next: bool| {
                                            values.with_mut(|m| {
                                                m.insert(key.clone(), if next { "true" } else { "false" }.into());
                                            });
                                        },
                                    }
                                    Label { r#for: Some(id.clone()),
                                        "{name}"
                                        if required {
                                            span { class: "text-destructive", " *" }
                                        }
                                        span { class: "font-normal text-muted-foreground", " · boolean" }
                                    }
                                }
                            } else {
                                Label { r#for: Some(id.clone()),
                                    "{name}"
                                    if required {
                                        span { class: "text-destructive", " *" }
                                    }
                                    span { class: "font-normal text-muted-foreground", " · {prop_type}" }
                                }
                                if is_json {
                                    Textarea {
                                        id: Some(id),
                                        rows: 3,
                                        value: current,
                                        placeholder: Some(if prop_type == "array" { "[]" } else { "{}" }.into()),
                                        oninput: move |e: FormEvent| {
                                            let v = e.value();
                                            values.with_mut(|m| {
                                                m.insert(key.clone(), v);
                                            });
                                        },
                                    }
                                } else {
                                    Input {
                                        id: Some(id),
                                        r#type: Some(input_ty.into()),
                                        value: current,
                                        required,
                                        placeholder: Some(name.clone()),
                                        oninput: move |e: FormEvent| {
                                            let v = e.value();
                                            values.with_mut(|m| {
                                                m.insert(key.clone(), v);
                                            });
                                        },
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
