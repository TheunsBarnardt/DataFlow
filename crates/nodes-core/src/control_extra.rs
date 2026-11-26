use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use serde_json::json;
use std::sync::Arc;

fn meta(ty: &str, name: &str, description: &str, outs: Vec<PortDef>) -> NodeMetadata {
    NodeMetadata {
        type_name: ty.into(),
        display_name: name.into(),
        category: "control".into(),
        description: description.into(),
        version: 1,
        is_trigger: false,
        inputs: vec![PortDef {
            id: "main".into(),
            name: "Main".into(),
        }],
        outputs: outs,
    }
}

pub struct SwitchNode;
#[async_trait]
impl NodeExecutor for SwitchNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "switch",
            "Switch",
            "Route items to different branches by matching rules",
            vec![
                PortDef {
                    id: "0".into(),
                    name: "0".into(),
                },
                PortDef {
                    id: "1".into(),
                    name: "1".into(),
                },
                PortDef {
                    id: "default".into(),
                    name: "Default".into(),
                },
            ],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let field = ctx
            .node
            .parameters
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rules = ctx
            .node
            .parameters
            .get("rules")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut handle = "default".to_string();
        let item = ctx.input_items.first().cloned().unwrap_or(json!({}));
        let val = if field.is_empty() {
            item.clone()
        } else {
            item.get(field).cloned().unwrap_or(serde_json::Value::Null)
        };
        for (i, rule) in rules.iter().enumerate() {
            let expected = rule.get("value").cloned().unwrap_or(serde_json::Value::Null);
            if val == expected {
                handle = i.to_string();
                break;
            }
        }
        Ok(NodeResult::branched(&handle, ctx.input_items))
    }
}

pub struct NoOpNode;
#[async_trait]
impl NodeExecutor for NoOpNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "noOp",
            "No Operation",
            "Pass items through unchanged",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        Ok(NodeResult::single(ctx.input_items))
    }
}

pub struct StopAndErrorNode;
#[async_trait]
impl NodeExecutor for StopAndErrorNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "stopAndError",
            "Stop and Error",
            "Stop the workflow and raise an error",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let msg = ctx
            .node
            .parameters
            .get("message")
            .or_else(|| ctx.node.parameters.get("errorMessage"))
            .and_then(|v| v.as_str())
            .unwrap_or("stopped by stopAndError node");
        Err(NodeError::msg(msg))
    }
}

pub struct StickyNoteNode;
#[async_trait]
impl NodeExecutor for StickyNoteNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "stickyNote",
            "Sticky Note",
            "Add a note on the canvas (does not affect data)",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        Ok(NodeResult::single(ctx.input_items))
    }
}

pub struct ExecuteCommandNode;
#[async_trait]
impl NodeExecutor for ExecuteCommandNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "executeCommand",
            "Execute Command",
            "Run a local shell command and capture output",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let cmd = ctx
            .node
            .parameters
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("command is required".into()))?;
        if std::env::var("ALLOW_EXECUTE_COMMAND").ok().as_deref() != Some("1") {
            return Err(NodeError::msg(
                "executeCommand disabled (set ALLOW_EXECUTE_COMMAND=1)",
            ));
        }
        let output = tokio::process::Command::new("cmd")
            .args(["/C", cmd])
            .output()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "code": output.status.code(),
        })]))
    }
}

pub struct ExecuteWorkflowNode;
#[async_trait]
impl NodeExecutor for ExecuteWorkflowNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "executeWorkflow",
            "Execute Sub-workflow",
            "Run another workflow and continue with its output",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let id = ctx
            .node
            .parameters
            .get("workflowId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(NodeResult::single(vec![json!({
            "workflowId": id,
            "items": ctx.input_items,
            "note": "sub-workflow dispatch is handled by application-services when wired",
        })]))
    }
}

pub struct McpCallNode;
#[async_trait]
impl NodeExecutor for McpCallNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "mcpCall",
            "MCP Call",
            "Invoke an MCP tool from a connected server",
            vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let tool = ctx
            .node
            .parameters
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(NodeResult::single(vec![json!({
            "tool": tool,
            "args": ctx.node.parameters.get("arguments").cloned().unwrap_or(json!({})),
            "stub": true,
        })]))
    }
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(SwitchNode));
    registry.register(Arc::new(NoOpNode));
    registry.register(Arc::new(StopAndErrorNode));
    registry.register(Arc::new(StickyNoteNode));
    registry.register(Arc::new(ExecuteCommandNode));
    registry.register(Arc::new(ExecuteWorkflowNode));
    registry.register(Arc::new(McpCallNode));
}
