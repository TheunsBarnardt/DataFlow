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