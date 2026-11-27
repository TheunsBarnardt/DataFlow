//! Full BotFlow node catalog mirrored from TypeScript `DAY_ONE_NODE_TYPES`.

use crate::{
    api_extra, cm_com, control_extra, data, files_extra, http, logic, microsoft, triggers,
    triggers_extra, wait,
};
use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use std::sync::Arc;

pub struct NodeSpec {
    pub type_name: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub is_trigger: bool,
}

pub const ALL_NODE_SPECS: &[NodeSpec] = &[
    NodeSpec { type_name: "webhook", display_name: "Webhook", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "schedule", display_name: "Schedule", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "manual", display_name: "Manual", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "rabbitmqTrigger", display_name: "RabbitMQ Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "redisTrigger", display_name: "Redis Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "outlookTrigger", display_name: "Microsoft Outlook Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "teamsTrigger", display_name: "Microsoft Teams Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "whatsappTrigger", display_name: "WhatsApp Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "formTrigger", display_name: "Form Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "errorTrigger", display_name: "Error Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "localFileTrigger", display_name: "Local File Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "sseTrigger", display_name: "SSE Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "rssFeedTrigger", display_name: "RSS Feed Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "emailTriggerImap", display_name: "Email Trigger (IMAP)", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "executeWorkflowTrigger", display_name: "Execute Workflow Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "workflowTrigger", display_name: "Workflow Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "n8nTrigger", display_name: "n8n Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "interval", display_name: "Interval", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "activationTrigger", display_name: "Activation Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "simulateTrigger", display_name: "Simulate Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "mcpServerTrigger", display_name: "MCP Server Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "evaluationTrigger", display_name: "Evaluation Trigger", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "apiEndpoint", display_name: "API Endpoint", category: "trigger", is_trigger: true },
    NodeSpec { type_name: "if", display_name: "IF", category: "control", is_trigger: false },
    NodeSpec { type_name: "switch", display_name: "Switch", category: "control", is_trigger: false },
    NodeSpec { type_name: "merge", display_name: "Merge", category: "control", is_trigger: false },
    NodeSpec { type_name: "wait", display_name: "Wait", category: "control", is_trigger: false },
    NodeSpec { type_name: "mcpCall", display_name: "MCP Call", category: "control", is_trigger: false },
    NodeSpec { type_name: "noOp", display_name: "No Operation", category: "control", is_trigger: false },
    NodeSpec { type_name: "stopAndError", display_name: "Stop and Error", category: "control", is_trigger: false },
    NodeSpec { type_name: "stickyNote", display_name: "Sticky Note", category: "control", is_trigger: false },
    NodeSpec { type_name: "executeWorkflow", display_name: "Execute Sub-workflow", category: "control", is_trigger: false },
    NodeSpec { type_name: "executeCommand", display_name: "Execute Command", category: "control", is_trigger: false },
    NodeSpec { type_name: "httpRequest", display_name: "HTTP Request", category: "api", is_trigger: false },
    NodeSpec { type_name: "cmCom", display_name: "CM.com", category: "api", is_trigger: false },
    NodeSpec { type_name: "webhookRespond", display_name: "Respond to Webhook", category: "api", is_trigger: false },
    NodeSpec { type_name: "apiRespond", display_name: "Respond to API", category: "api", is_trigger: false },
    NodeSpec { type_name: "graphql", display_name: "GraphQL", category: "api", is_trigger: false },
    NodeSpec { type_name: "jwt", display_name: "JWT", category: "api", is_trigger: false },
    NodeSpec { type_name: "n8nApi", display_name: "n8n", category: "api", is_trigger: false },
    NodeSpec { type_name: "form", display_name: "Form", category: "api", is_trigger: false },
    NodeSpec { type_name: "rssRead", display_name: "RSS Read", category: "api", is_trigger: false },
    NodeSpec { type_name: "sendEmail", display_name: "Send Email", category: "api", is_trigger: false },
    NodeSpec { type_name: "ftp", display_name: "FTP", category: "api", is_trigger: false },
    NodeSpec { type_name: "ssh", display_name: "SSH", category: "api", is_trigger: false },
    NodeSpec { type_name: "git", display_name: "Git", category: "api", is_trigger: false },
    NodeSpec { type_name: "ldap", display_name: "LDAP", category: "api", is_trigger: false },
    NodeSpec { type_name: "mssql", display_name: "MSSQL", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "microsoftGraph", display_name: "Microsoft Graph", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "outlook", display_name: "Outlook", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "excelOnline", display_name: "Excel Online", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "sharepoint", display_name: "SharePoint", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "dynamics", display_name: "Dataverse", category: "microsoft", is_trigger: false },
    NodeSpec { type_name: "readFile", display_name: "Read File", category: "files", is_trigger: false },
    NodeSpec { type_name: "writeFile", display_name: "Write File", category: "files", is_trigger: false },
    NodeSpec { type_name: "listFiles", display_name: "List Files", category: "files", is_trigger: false },
    NodeSpec { type_name: "moveFile", display_name: "Move File", category: "files", is_trigger: false },
    NodeSpec { type_name: "copyFile", display_name: "Copy File", category: "files", is_trigger: false },
    NodeSpec { type_name: "deleteFile", display_name: "Delete File", category: "files", is_trigger: false },
    NodeSpec { type_name: "csv", display_name: "CSV", category: "files", is_trigger: false },
    NodeSpec { type_name: "json", display_name: "JSON", category: "files", is_trigger: false },
    NodeSpec { type_name: "xml", display_name: "XML", category: "files", is_trigger: false },
    NodeSpec { type_name: "excel", display_name: "Excel File", category: "files", is_trigger: false },
    NodeSpec { type_name: "zip", display_name: "Zip", category: "files", is_trigger: false },
    NodeSpec { type_name: "compression", display_name: "Compression", category: "files", is_trigger: false },
    NodeSpec { type_name: "convertToFile", display_name: "Convert to File", category: "files", is_trigger: false },
    NodeSpec { type_name: "extractFromFile", display_name: "Extract from File", category: "files", is_trigger: false },
    NodeSpec { type_name: "readPdf", display_name: "Read PDF", category: "files", is_trigger: false },
    NodeSpec { type_name: "moveBinaryData", display_name: "Move Binary Data", category: "files", is_trigger: false },
    NodeSpec { type_name: "editImage", display_name: "Edit Image", category: "files", is_trigger: false },
    NodeSpec { type_name: "icalendar", display_name: "iCalendar", category: "files", is_trigger: false },
    NodeSpec { type_name: "set", display_name: "Edit Fields (Set)", category: "data", is_trigger: false },
    NodeSpec { type_name: "editFields", display_name: "Edit Fields", category: "data", is_trigger: false },
    NodeSpec { type_name: "filter", display_name: "Filter", category: "data", is_trigger: false },
    NodeSpec { type_name: "sort", display_name: "Sort", category: "data", is_trigger: false },
    NodeSpec { type_name: "limit", display_name: "Limit", category: "data", is_trigger: false },
    NodeSpec { type_name: "splitInBatches", display_name: "Loop Over Items", category: "data", is_trigger: false },
    NodeSpec { type_name: "splitOut", display_name: "Split Out", category: "data", is_trigger: false },
    NodeSpec { type_name: "aggregate", display_name: "Aggregate", category: "data", is_trigger: false },
    NodeSpec { type_name: "summarize", display_name: "Summarize", category: "data", is_trigger: false },
    NodeSpec { type_name: "itemLists", display_name: "Item Lists", category: "data", is_trigger: false },
    NodeSpec { type_name: "renameKeys", display_name: "Rename Keys", category: "data", is_trigger: false },
    NodeSpec { type_name: "removeDuplicates", display_name: "Remove Duplicates", category: "data", is_trigger: false },
    NodeSpec { type_name: "compareDatasets", display_name: "Compare Datasets", category: "data", is_trigger: false },
    NodeSpec { type_name: "code", display_name: "Code", category: "data", is_trigger: false },
    NodeSpec { type_name: "aiTransform", display_name: "AI Transform", category: "data", is_trigger: false },
    NodeSpec { type_name: "dateTime", display_name: "Date & Time", category: "data", is_trigger: false },
    NodeSpec { type_name: "crypto", display_name: "Crypto", category: "data", is_trigger: false },
    NodeSpec { type_name: "html", display_name: "HTML", category: "data", is_trigger: false },
    NodeSpec { type_name: "htmlExtract", display_name: "HTML Extract", category: "data", is_trigger: false },
    NodeSpec { type_name: "markdown", display_name: "Markdown", category: "data", is_trigger: false },
    NodeSpec { type_name: "totp", display_name: "TOTP", category: "data", is_trigger: false },
    NodeSpec { type_name: "timeSaved", display_name: "Track Time Saved", category: "data", is_trigger: false },
    NodeSpec { type_name: "dataTable", display_name: "Data Table", category: "data", is_trigger: false },
    NodeSpec { type_name: "executionData", display_name: "Execution Data", category: "data", is_trigger: false },
    NodeSpec { type_name: "simulate", display_name: "Simulate", category: "data", is_trigger: false },
    NodeSpec { type_name: "debugHelper", display_name: "Debug Helper", category: "data", is_trigger: false },
    NodeSpec { type_name: "evaluation", display_name: "Evaluation", category: "data", is_trigger: false },
    NodeSpec { type_name: "postgres", display_name: "PostgreSQL", category: "database", is_trigger: false },
    NodeSpec { type_name: "llm", display_name: "LLM", category: "ai", is_trigger: false },
    NodeSpec { type_name: "approval", display_name: "Human Approval", category: "ai", is_trigger: false },
];

pub struct PassThroughNode {
    pub spec: &'static NodeSpec,
}

#[async_trait]
impl NodeExecutor for PassThroughNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: self.spec.type_name.into(),
            display_name: self.spec.display_name.into(),
            category: self.spec.category.into(),
            description: format!("{} (Rust)", self.spec.display_name),
            version: 1,
            is_trigger: self.spec.is_trigger,
            inputs: if self.spec.is_trigger {
                vec![]
            } else {
                vec![PortDef {
                    id: "main".into(),
                    name: "Main".into(),
                }]
            },
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

pub fn register_specialized(registry: &mut NodeRegistry) {
    // Core originals
    registry.register(Arc::new(triggers::ManualTrigger));
    registry.register(Arc::new(triggers::WebhookTrigger));
    registry.register(Arc::new(triggers::ScheduleTrigger));
    registry.register(Arc::new(http::HttpRequestNode::default()));
    registry.register(Arc::new(logic::SetNode));
    registry.register(Arc::new(logic::IfNode));
    registry.register(Arc::new(logic::MergeNode));
    registry.register(Arc::new(wait::WaitNode));

    triggers_extra::register(registry);
    control_extra::register(registry);
    data::register(registry);
    files_extra::register(registry);
    api_extra::register(registry);
    microsoft::register(registry);
    cm_com::register(registry);
}

pub fn fill_missing_from_catalog(registry: &mut NodeRegistry) {
    for spec in ALL_NODE_SPECS {
        if registry.get(spec.type_name).is_none() {
            registry.register(Arc::new(PassThroughNode { spec }));
        }
    }
}

/// Register every BotFlow node (specialized + catalog fill).
pub fn register_all_nodes(registry: &mut NodeRegistry) {
    register_specialized(registry);
    fill_missing_from_catalog(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_day_one_types_registered() {
        let mut reg = NodeRegistry::new();
        register_all_nodes(&mut reg);
        assert!(
            ALL_NODE_SPECS.len() >= 97,
            "expected full TS catalog, got {}",
            ALL_NODE_SPECS.len()
        );
        for spec in ALL_NODE_SPECS {
            assert!(
                reg.get(spec.type_name).is_some(),
                "missing {}",
                spec.type_name
            );
        }
        assert!(reg.list_metadata().len() >= 97);
    }
}
