use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use std::sync::Arc;

macro_rules! trigger {
    ($struct:ident, $ty:expr, $name:expr) => {
        pub struct $struct;
        #[async_trait]
        impl NodeExecutor for $struct {
            fn metadata(&self) -> NodeMetadata {
                NodeMetadata {
                    type_name: $ty.into(),
                    display_name: $name.into(),
                    category: "trigger".into(),
                    description: $name.into(),
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
    };
}

trigger!(RabbitmqTrigger, "rabbitmqTrigger", "RabbitMQ Trigger");
trigger!(RedisTrigger, "redisTrigger", "Redis Trigger");
trigger!(OutlookTrigger, "outlookTrigger", "Microsoft Outlook Trigger");
trigger!(TeamsTrigger, "teamsTrigger", "Microsoft Teams Trigger");
trigger!(WhatsappTrigger, "whatsappTrigger", "WhatsApp Trigger");
trigger!(FormTrigger, "formTrigger", "Form Trigger");
trigger!(ErrorTrigger, "errorTrigger", "Error Trigger");
trigger!(LocalFileTrigger, "localFileTrigger", "Local File Trigger");
trigger!(SseTrigger, "sseTrigger", "SSE Trigger");
trigger!(RssFeedTrigger, "rssFeedTrigger", "RSS Feed Trigger");
trigger!(EmailTriggerImap, "emailTriggerImap", "Email Trigger (IMAP)");
trigger!(ExecuteWorkflowTrigger, "executeWorkflowTrigger", "Execute Workflow Trigger");
trigger!(WorkflowTrigger, "workflowTrigger", "Workflow Trigger");
trigger!(N8nTrigger, "n8nTrigger", "n8n Trigger");
trigger!(IntervalTrigger, "interval", "Interval");
trigger!(ActivationTrigger, "activationTrigger", "Activation Trigger");
trigger!(SimulateTrigger, "simulateTrigger", "Simulate Trigger");
trigger!(McpServerTrigger, "mcpServerTrigger", "MCP Server Trigger");
trigger!(EvaluationTrigger, "evaluationTrigger", "Evaluation Trigger");
trigger!(ApiEndpointTrigger, "apiEndpoint", "API Endpoint");

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(RabbitmqTrigger));
    registry.register(Arc::new(RedisTrigger));
    registry.register(Arc::new(OutlookTrigger));
    registry.register(Arc::new(TeamsTrigger));
    registry.register(Arc::new(WhatsappTrigger));
    registry.register(Arc::new(FormTrigger));
    registry.register(Arc::new(ErrorTrigger));
    registry.register(Arc::new(LocalFileTrigger));
    registry.register(Arc::new(SseTrigger));
    registry.register(Arc::new(RssFeedTrigger));
    registry.register(Arc::new(EmailTriggerImap));
    registry.register(Arc::new(ExecuteWorkflowTrigger));
    registry.register(Arc::new(WorkflowTrigger));
    registry.register(Arc::new(N8nTrigger));
    registry.register(Arc::new(IntervalTrigger));
    registry.register(Arc::new(ActivationTrigger));
    registry.register(Arc::new(SimulateTrigger));
    registry.register(Arc::new(McpServerTrigger));
    registry.register(Arc::new(EvaluationTrigger));
    registry.register(Arc::new(ApiEndpointTrigger));
}
