use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};

pub struct ManualTrigger;

#[async_trait]
impl NodeExecutor for ManualTrigger {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "manual".into(),
            display_name: "Manual Trigger".into(),
            category: "trigger".into(),
            description: "Start a workflow from the UI or API".into(),
            version: 1,
            is_trigger: true,
            inputs: vec![],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        Ok(NodeResult::single(ctx.input_items))
    }
}

pub struct WebhookTrigger;

#[async_trait]
impl NodeExecutor for WebhookTrigger {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "webhook".into(),
            display_name: "Webhook".into(),
            category: "trigger".into(),
            description: "Start from an inbound HTTP webhook".into(),
            version: 1,
            is_trigger: true,
            inputs: vec![],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        Ok(NodeResult::single(ctx.input_items))
    }
}

pub struct ScheduleTrigger;

#[async_trait]
impl NodeExecutor for ScheduleTrigger {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "schedule".into(),
            display_name: "Schedule".into(),
            category: "trigger".into(),
            description: "Periodic trigger".into(),
            version: 1,
            is_trigger: true,
            inputs: vec![],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        Ok(NodeResult::single(ctx.input_items))
    }
}
