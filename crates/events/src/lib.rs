use botflow_workflow_model::RunStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    pub run_id: String,
    pub workflow_id: String,
    pub trace_id: String,
    pub kind: RunEventKind,
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub status: Option<RunStatus>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunEventKind {
    RunStatus,
    NodeStatus,
    Log,
    Progress,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<RunEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: RunEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RunEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
