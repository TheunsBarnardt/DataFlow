use botflow_workflow_model::{NodeRunState, RunDoc, RunStatus, WorkflowDoc};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RuntimeLimits {
    pub max_concurrent_nodes: usize,
    pub default_node_timeout_ms: u64,
    pub max_payload_bytes: usize,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            max_concurrent_nodes: 8,
            default_node_timeout_ms: 60_000,
            max_payload_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunHandle {
    pub run: Arc<Mutex<RunDoc>>,
    pub cancel: CancellationToken,
    pub finished: Arc<Notify>,
}

impl RunHandle {
    pub fn new(workflow: &WorkflowDoc, input: Value) -> Self {
        let trace_id = Uuid::new_v4().to_string();
        let run = RunDoc {
            id: Some(Uuid::new_v4().to_string()),
            workflow_id: workflow.id.clone().unwrap_or_default(),
            workflow_version: workflow.version,
            status: RunStatus::Created,
            input,
            output: None,
            error: None,
            node_states: workflow
                .nodes
                .iter()
                .map(|n| NodeRunState {
                    node_id: n.id.clone(),
                    node_name: Some(n.name.clone()),
                    status: RunStatus::Created,
                    started_at: None,
                    finished_at: None,
                    input: None,
                    output: None,
                    error: None,
                    attempt: 0,
                })
                .collect(),
            workflow_name: Some(workflow.name.clone()),
            trigger: Some("manual".into()),
            actor: None,
            time_saved_minutes: None,
            trace_id,
            started_at: Utc::now(),
            finished_at: None,
        };
        Self {
            run: Arc::new(Mutex::new(run)),
            cancel: CancellationToken::new(),
            finished: Arc::new(Notify::new()),
        }
    }

    pub async fn status(&self) -> RunStatus {
        self.run.lock().await.status
    }

    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }
}

#[derive(Default)]
pub struct NodeOutputStore {
    /// node_id -> aggregated items from successful execution
    pub by_node: HashMap<String, Value>,
    /// node_id -> handle -> items (for branching)
    pub by_handle: HashMap<String, HashMap<String, Vec<Value>>>,
}

impl NodeOutputStore {
    pub fn set(&mut self, node_id: &str, handle: Option<&str>, items: Vec<Value>) {
        let value = Value::Array(items.clone());
        self.by_node.insert(node_id.to_string(), value);
        if let Some(h) = handle {
            self.by_handle
                .entry(node_id.to_string())
                .or_default()
                .insert(h.to_string(), items);
        }
    }

    pub fn get_items(&self, node_id: &str) -> Vec<Value> {
        match self.by_node.get(node_id) {
            Some(Value::Array(arr)) => arr.clone(),
            Some(other) => vec![other.clone()],
            None => vec![],
        }
    }
}
