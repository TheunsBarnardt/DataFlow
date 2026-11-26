use async_trait::async_trait;
use botflow_workflow_model::{WorkflowEdge, WorkflowNode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetadata {
    pub type_name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub version: u32,
    pub is_trigger: bool,
    #[serde(default)]
    pub inputs: Vec<PortDef>,
    #[serde(default)]
    pub outputs: Vec<PortDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortDef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub run_id: String,
    pub workflow_id: String,
    pub trace_id: String,
    pub node: WorkflowNode,
    pub input_items: Vec<Value>,
    pub cancel: CancellationToken,
    pub timeout: Duration,
    pub credentials: HashMap<String, Value>,
    pub variables: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeOutput {
    pub items: Vec<Value>,
    #[serde(default)]
    pub paired_item: Option<usize>,
    /// Branch key for IF/Switch (e.g. "true" / "false")
    #[serde(default)]
    pub output_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeResult {
    pub outputs: Vec<NodeOutput>,
}

impl NodeResult {
    pub fn single(items: Vec<Value>) -> Self {
        Self {
            outputs: vec![NodeOutput {
                items,
                paired_item: None,
                output_handle: None,
            }],
        }
    }

    pub fn branched(handle: &str, items: Vec<Value>) -> Self {
        Self {
            outputs: vec![NodeOutput {
                items,
                paired_item: None,
                output_handle: Some(handle.to_string()),
            }],
        }
    }
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("{0}")]
    Message(String),
    #[error("cancelled")]
    Cancelled,
    #[error("timeout")]
    Timeout,
    #[error("configuration: {0}")]
    Config(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl NodeError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

#[async_trait]
pub trait NodeExecutor: Send + Sync {
    fn metadata(&self) -> NodeMetadata;
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError>;
}

#[derive(Default, Clone)]
pub struct NodeRegistry {
    executors: HashMap<String, Arc<dyn NodeExecutor>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, executor: Arc<dyn NodeExecutor>) {
        let key = executor.metadata().type_name.clone();
        self.executors.insert(key, executor);
    }

    pub fn get(&self, type_name: &str) -> Option<Arc<dyn NodeExecutor>> {
        self.executors.get(type_name).cloned()
    }

    pub fn list_metadata(&self) -> Vec<NodeMetadata> {
        self.executors.values().map(|e| e.metadata()).collect()
    }

    pub fn resolve_for_node(&self, node: &WorkflowNode) -> Result<Arc<dyn NodeExecutor>, NodeError> {
        self.get(&node.node_type)
            .ok_or_else(|| NodeError::msg(format!("unknown node type: {}", node.node_type)))
    }
}

pub fn outgoing_edges<'a>(
    edges: &'a [WorkflowEdge],
    source_id: &str,
    handle: Option<&str>,
) -> Vec<&'a WorkflowEdge> {
    edges
        .iter()
        .filter(|e| {
            e.source == source_id
                && match handle {
                    Some(h) => e.source_handle.as_deref() == Some(h) || e.source_handle.is_none(),
                    None => true,
                }
        })
        .collect()
}

// Re-export anyhow for NodeError
pub use anyhow;
