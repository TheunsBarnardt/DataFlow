mod binary_viewer;
mod webhook_listen_panel;
mod condition_builder;
mod credentials_picker;
mod dashboard;
mod expression_field;
mod ndv_data_pane;
mod field_control;
mod json_schema_properties_editor;
mod json_schema_sample;
mod key_value_editor;
mod node_details;
mod switch_rules_editor;
mod variable_aware_field;
mod workflow_list;
mod api_projects;
mod variables;
mod credentials;
mod nodes;
mod runs;
mod insights;
mod audit;
mod login;
mod chat;

pub use binary_viewer::{has_binary_data, BinaryViewer};
pub use ndv_data_pane::{normalize_drag_expr, DF_DRAG_MIME};
pub use api_projects::ApiProjectsPanel;
pub use audit::AuditPanel;
pub use chat::ChatPanel;
pub use credentials::{CredentialDialog, CredentialsPanel};
pub use dashboard::DashboardPanel;
pub use insights::InsightsPanel;
pub use login::LoginPage;
pub use node_details::{NodeDetailsPanel, NodeUpdate};
pub use nodes::{NodeCatalogItem, NodesPanel, NodesPanelVariant};
pub use runs::RunsPanel;
pub use variables::VariablesPanel;
pub use workflow_list::WorkflowList;
pub use json_schema_sample::sample_from_api_endpoint_parameters;
pub use webhook_listen_panel::{webhook_sample_from_value, WebhookListenPanel, WebhookSample};

pub(crate) fn json_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub(crate) fn json_id(v: &serde_json::Value) -> String {
    v.get("_id")
        .or_else(|| v.get("id"))
        .map(json_string)
        .unwrap_or_default()
}

pub(crate) fn json_bool(v: &serde_json::Value, key: &str, default: bool) -> bool {
    v.get(key)
        .and_then(|x| x.as_bool())
        .unwrap_or(default)
}

/// Parse an API ISO timestamp (UTC) into the system local timezone.
pub(crate) fn parse_local_datetime(raw: &str) -> Option<chrono::DateTime<chrono::Local>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&chrono::Local));
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(raw, fmt) {
            return Some(ndt.and_utc().with_timezone(&chrono::Local));
        }
    }
    None
}

/// Human-readable local datetime, e.g. `2026-11-26 12:28:04`.
pub(crate) fn format_local_datetime(raw: &str) -> String {
    match parse_local_datetime(raw) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None if raw.is_empty() => String::new(),
        None => raw.replace('T', " ").trim_end_matches('Z').to_string(),
    }
}

/// Short local datetime, e.g. `Nov 26, 2026 12:28`.
pub(crate) fn format_local_datetime_short(raw: &str) -> String {
    match parse_local_datetime(raw) {
        Some(dt) => dt.format("%b %d, %Y %H:%M").to_string(),
        None if raw.is_empty() => String::new(),
        None => raw.to_string(),
    }
}