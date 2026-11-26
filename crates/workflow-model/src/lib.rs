use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;

fn deserialize_dt<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(s) => DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&Utc))
            .or_else(|_| {
                // JS often drops timezone offset formatting edge cases
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.fZ")
                    .map(|n| DateTime::<Utc>::from_naive_utc_and_offset(n, Utc))
            })
            .map_err(serde::de::Error::custom),
        Value::Number(n) => {
            let ms = n.as_i64().unwrap_or(0);
            DateTime::from_timestamp_millis(ms).ok_or_else(|| serde::de::Error::custom("bad epoch"))
        }
        other => {
            // BSON Date often becomes {"$date": "..."} via some paths
            if let Some(s) = other.get("$date").and_then(|x| x.as_str()) {
                return DateTime::parse_from_rfc3339(s)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(serde::de::Error::custom);
            }
            Err(serde::de::Error::custom(format!(
                "expected datetime, got {other}"
            )))
        }
    }
}

fn deserialize_opt_dt<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<Value>::deserialize(deserializer)?;
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(val) => {
            // reuse by serializing back — small helper inline
            match val {
                Value::String(s) => DateTime::parse_from_rfc3339(&s)
                    .map(|d| Some(d.with_timezone(&Utc)))
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.fZ")
                            .map(|n| Some(DateTime::<Utc>::from_naive_utc_and_offset(n, Utc)))
                    })
                    .map_err(serde::de::Error::custom),
                Value::Number(n) => Ok(DateTime::from_timestamp_millis(n.as_i64().unwrap_or(0))),
                other => {
                    if let Some(s) = other.get("$date").and_then(|x| x.as_str()) {
                        DateTime::parse_from_rfc3339(s)
                            .map(|d| Some(d.with_timezone(&Utc)))
                            .map_err(serde::de::Error::custom)
                    } else if let Some(n) = other.get("$date").and_then(|x| x.as_i64()) {
                        Ok(DateTime::from_timestamp_millis(n))
                    } else {
                        Err(serde::de::Error::custom(format!(
                            "expected datetime, got {other}"
                        )))
                    }
                }
            }
        }
    }
}

pub const WORKFLOW_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NodeType {
    Manual,
    Webhook,
    Schedule,
    HttpRequest,
    Set,
    If,
    Merge,
    Wait,
    Mssql,
    Postgres,
    SendEmail,
    ReadFile,
    WriteFile,
    Llm,
    Approval,
    #[serde(other)]
    Unknown,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Webhook => "webhook",
            Self::Schedule => "schedule",
            Self::HttpRequest => "httpRequest",
            Self::Set => "set",
            Self::If => "if",
            Self::Merge => "merge",
            Self::Wait => "wait",
            Self::Mssql => "mssql",
            Self::Postgres => "postgres",
            Self::SendEmail => "sendEmail",
            Self::ReadFile => "readFile",
            Self::WriteFile => "writeFile",
            Self::Llm => "llm",
            Self::Approval => "approval",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeSettings {
    pub always_output_data: Option<bool>,
    pub execute_once: Option<bool>,
    pub retry_on_fail: Option<bool>,
    pub max_tries: Option<u32>,
    pub wait_between_tries: Option<u64>,
    pub on_error: Option<String>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub name: String,
    pub position: Position,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub settings: Option<NodeSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub source_handle: Option<String>,
    #[serde(default)]
    pub target_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default = "default_workflow_name")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_version_one")]
    pub version: u32,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Minutes saved per successful production run (ROI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_saved_minutes: Option<f64>,
    /// Default workflow; `apiEndpoint` belongs to an ApiProject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Parent API project when kind === "apiEndpoint".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_workflow_name() -> String {
    "My workflow".into()
}

fn default_true() -> bool {
    true
}

fn default_version_one() -> u32 {
    1
}

fn default_schema_version() -> u32 {
    WORKFLOW_SCHEMA_VERSION
}

/// Aligns with TypeScript `RunStatus` (`completed` not `succeeded`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Created,
    Queued,
    Running,
    Waiting,
    Retrying,
    Succeeded,
    Failed,
    Cancelled,
}

impl Serialize for RunStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = match self {
            Self::Created => "created",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Retrying => "retrying",
            Self::Succeeded => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for RunStatus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "created" => Self::Created,
            "queued" => Self::Queued,
            "running" => Self::Running,
            "waiting" => Self::Waiting,
            "retrying" => Self::Retrying,
            "completed" | "succeeded" | "success" => Self::Succeeded,
            "failed" | "error" => Self::Failed,
            "cancelled" | "canceled" => Self::Cancelled,
            other => {
                return Err(serde::de::Error::unknown_variant(
                    other,
                    &[
                        "created",
                        "queued",
                        "running",
                        "waiting",
                        "retrying",
                        "completed",
                        "failed",
                        "cancelled",
                    ],
                ))
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRunState {
    pub node_id: String,
    #[serde(default)]
    pub node_name: Option<String>,
    pub status: RunStatus,
    #[serde(default, deserialize_with = "deserialize_opt_dt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_opt_dt")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub workflow_id: String,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub workflow_version: u32,
    pub status: RunStatus,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
    /// Rust scheduler state (also populated from TS `logs` on read).
    #[serde(default, alias = "logs")]
    pub node_states: Vec<NodeRunState>,
    #[serde(default)]
    pub trace_id: String,
    #[serde(deserialize_with = "deserialize_dt")]
    pub started_at: DateTime<Utc>,
    #[serde(default, deserialize_with = "deserialize_opt_dt")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub time_saved_minutes: Option<f64>,
}

impl RunDoc {
    /// JSON shape expected by the React UI (`logs` + `completed` status).
    pub fn to_ui_json(&self) -> Value {
        let logs: Vec<Value> = self
            .node_states
            .iter()
            .map(|n| {
                serde_json::json!({
                    "nodeId": n.node_id,
                    "nodeName": n.node_name.clone().unwrap_or_else(|| n.node_id.clone()),
                    "status": n.status,
                    "startedAt": n.started_at,
                    "finishedAt": n.finished_at,
                    "input": n.input,
                    "output": n.output,
                    "error": n.error,
                })
            })
            .collect();
        serde_json::json!({
            "_id": self.id,
            "workflowId": self.workflow_id,
            "workflowName": self.workflow_name.clone().unwrap_or_default(),
            "status": self.status,
            "trigger": self.trigger.clone().unwrap_or_else(|| "manual".into()),
            "startedAt": self.started_at,
            "finishedAt": self.finished_at,
            "input": self.input,
            "output": self.output,
            "error": self.error,
            "logs": logs,
            "actor": self.actor,
            "timeSavedMinutes": self.time_saved_minutes,
            "traceId": self.trace_id,
            "workflowVersion": self.workflow_version,
        })
    }
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl WorkflowDoc {
    pub fn from_json(value: &Value) -> Result<Self, ModelError> {
        Ok(serde_json::from_value(value.clone())?)
    }

    pub fn to_json(&self) -> Result<Value, ModelError> {
        Ok(serde_json::to_value(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roundtrip_minimal_workflow() {
        let v = json!({
            "name": "demo",
            "enabled": true,
            "version": 1,
            "nodes": [{
                "id": "n1",
                "type": "manual",
                "name": "Start",
                "position": {"x": 0, "y": 0},
                "parameters": {}
            }],
            "edges": []
        });
        let doc = WorkflowDoc::from_json(&v).unwrap();
        assert_eq!(doc.name, "demo");
        assert_eq!(doc.nodes.len(), 1);
        let back = doc.to_json().unwrap();
        assert_eq!(back["name"], "demo");
    }
}
