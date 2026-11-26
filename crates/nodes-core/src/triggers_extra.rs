use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use std::sync::Arc;

macro_rules! trigger {
    ($struct:ident, $ty:expr, $name:expr, $desc:expr) => {
        pub struct $struct;
        #[async_trait]
        impl NodeExecutor for $struct {
            fn metadata(&self) -> NodeMetadata {
                NodeMetadata {
                    type_name: $ty.into(),
                    display_name: $name.into(),
                    category: "trigger".into(),
                    description: $desc.into(),
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

trigger!(
    RabbitmqTrigger,
    "rabbitmqTrigger",
    "RabbitMQ Trigger",
    "Start when a message arrives on a RabbitMQ queue"
);
trigger!(
    RedisTrigger,
    "redisTrigger",
    "Redis Trigger",
    "Start when a Redis key event or pub/sub message is received"
);
trigger!(
    OutlookTrigger,
    "outlookTrigger",
    "Microsoft Outlook Trigger",
    "Start when a new Outlook email arrives"
);
trigger!(
    TeamsTrigger,
    "teamsTrigger",
    "Microsoft Teams Trigger",
    "Start when a Teams message or activity is received"
);
trigger!(
    WhatsappTrigger,
    "whatsappTrigger",
    "WhatsApp Trigger",
    "Start when a WhatsApp message is received"
);
trigger!(
    FormTrigger,
    "formTrigger",
    "Form Trigger",
    "Start when a form is submitted"
);
trigger!(
    ErrorTrigger,
    "errorTrigger",
    "Error Trigger",
    "Start when another workflow run fails"
);
trigger!(
    LocalFileTrigger,
    "localFileTrigger",
    "Local File Trigger",
    "Start when a file is created or changed on disk"
);
trigger!(
    SseTrigger,
    "sseTrigger",
    "SSE Trigger",
    "Start from a Server-Sent Events stream"
);
trigger!(
    RssFeedTrigger,
    "rssFeedTrigger",
    "RSS Feed Trigger",
    "Start when a new item appears in an RSS feed"
);
trigger!(
    EmailTriggerImap,
    "emailTriggerImap",
    "Email Trigger (IMAP)",
    "Start when new mail arrives via IMAP"
);
trigger!(
    ExecuteWorkflowTrigger,
    "executeWorkflowTrigger",
    "Execute Workflow Trigger",
    "Start when called from another workflow"
);
trigger!(
    WorkflowTrigger,
    "workflowTrigger",
    "Workflow Trigger",
    "Start when another workflow signals this one"
);
trigger!(
    N8nTrigger,
    "n8nTrigger",
    "n8n Trigger",
    "Start from an n8n workflow event"
);
trigger!(
    IntervalTrigger,
    "interval",
    "Interval",
    "Start repeatedly on a fixed time interval"
);
trigger!(
    ActivationTrigger,
    "activationTrigger",
    "Activation Trigger",
    "Start when the workflow is activated"
);
trigger!(
    SimulateTrigger,
    "simulateTrigger",
    "Simulate Trigger",
    "Start with sample data for testing"
);
trigger!(
    McpServerTrigger,
    "mcpServerTrigger",
    "MCP Server Trigger",
    "Start when an MCP client invokes this workflow"
);
trigger!(
    EvaluationTrigger,
    "evaluationTrigger",
    "Evaluation Trigger",
    "Start an evaluation run for testing quality"
);
trigger!(
    ApiEndpointTrigger,
    "apiEndpoint",
    "API Endpoint",
    "Expose an HTTP endpoint that starts this workflow"
);

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
