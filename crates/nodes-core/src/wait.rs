use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use std::time::Duration;

pub struct WaitNode;

#[async_trait]
impl NodeExecutor for WaitNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "wait".into(),
            display_name: "Wait".into(),
            category: "flow".into(),
            description: "Pause for a duration (in-memory; durable wait later)".into(),
            version: 1,
            is_trigger: false,
            inputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let ms = ctx
            .node
            .parameters
            .get("amount")
            .or_else(|| ctx.node.parameters.get("ms"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);
        tokio::select! {
            _ = ctx.cancel.cancelled() => return Err(NodeError::Cancelled),
            _ = tokio::time::sleep(Duration::from_millis(ms)) => {}
        }
        Ok(NodeResult::single(ctx.input_items))
    }
}
