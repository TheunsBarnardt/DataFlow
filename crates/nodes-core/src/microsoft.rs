use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use serde_json::json;
use std::sync::Arc;

fn meta(ty: &str, name: &str, description: &str) -> NodeMetadata {
    NodeMetadata {
        type_name: ty.into(),
        display_name: name.into(),
        category: "microsoft".into(),
        description: description.into(),
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

async fn graph_get(path: &str) -> Result<serde_json::Value, NodeError> {
    let token = std::env::var("MS_GRAPH_TOKEN")
        .map_err(|_| NodeError::Config("MS_GRAPH_TOKEN required".into()))?;
    let url = if path.starts_with("http") {
        path.to_string()
    } else {
        format!("https://graph.microsoft.com/v1.0{}", path)
    };
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| NodeError::msg(e.to_string()))?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
    if !status.is_success() {
        return Err(NodeError::msg(format!("graph {status}: {body}")));
    }
    Ok(body)
}

pub struct MicrosoftGraphNode;
#[async_trait]
impl NodeExecutor for MicrosoftGraphNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "microsoftGraph",
            "Microsoft Graph",
            "Call Microsoft Graph REST endpoints",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let path = ctx
            .node
            .parameters
            .get("path")
            .or_else(|| ctx.node.parameters.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("/me");
        let body = graph_get(path).await?;
        Ok(NodeResult::single(vec![body]))
    }
}

pub struct OutlookNode;
#[async_trait]
impl NodeExecutor for OutlookNode {
    fn metadata(&self) -> NodeMetadata {
        meta("outlook", "Outlook", "Read or send Outlook mail via Graph")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let op = ctx
            .node
            .parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("listMessages");
        let path = match op {
            "listMessages" => "/me/messages?$top=10",
            _ => "/me/mailFolders",
        };
        let body = graph_get(path).await.unwrap_or_else(|e| {
            json!({ "error": e.to_string(), "operation": op, "items": ctx.input_items })
        });
        Ok(NodeResult::single(vec![body]))
    }
}

pub struct ExcelOnlineNode;
#[async_trait]
impl NodeExecutor for ExcelOnlineNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "excelOnline",
            "Excel Online",
            "Work with Excel workbooks in OneDrive or SharePoint",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let path = ctx
            .node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/me/drive/root/children");
        let body = graph_get(path).await.unwrap_or_else(|e| json!({ "error": e.to_string() }));
        Ok(NodeResult::single(vec![body]))
    }
}

pub struct SharepointNode;
#[async_trait]
impl NodeExecutor for SharepointNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "sharepoint",
            "SharePoint",
            "Access SharePoint sites, lists, and files",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let site = ctx
            .node
            .parameters
            .get("siteId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path = if site.is_empty() {
            "/sites?search=*".to_string()
        } else {
            format!("/sites/{site}")
        };
        let body = graph_get(&path)
            .await
            .unwrap_or_else(|e| json!({ "error": e.to_string() }));
        Ok(NodeResult::single(vec![body]))
    }
}

pub struct DynamicsNode;
#[async_trait]
impl NodeExecutor for DynamicsNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "dynamics",
            "Dataverse",
            "Query or update Dataverse / Dynamics 365 records",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let base = std::env::var("DYNAMICS_BASE_URL")
            .or_else(|_| std::env::var("DYNAMICS_API_BASE_URL"))
            .unwrap_or_default();
        let entity = ctx
            .node
            .parameters
            .get("entity")
            .and_then(|v| v.as_str())
            .unwrap_or("accounts");
        if base.is_empty() {
            return Ok(NodeResult::single(vec![json!({
                "error": "DYNAMICS_BASE_URL not set",
                "entity": entity,
                "items": ctx.input_items,
            })]));
        }
        let token = std::env::var("DYNAMICS_TOKEN").unwrap_or_default();
        let url = format!(
            "{}/api/data/v9.2/{}",
            base.trim_end_matches('/'),
            entity
        );
        let mut req = reqwest::Client::new().get(&url);
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
        if let Ok(key) = std::env::var("DYNAMICS_API_KEY") {
            req = req.header("Ocp-Apim-Subscription-Key", key);
        }
        let resp = req.send().await.map_err(|e| NodeError::msg(e.to_string()))?;
        let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
        Ok(NodeResult::single(vec![body]))
    }
}

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(MicrosoftGraphNode));
    registry.register(Arc::new(OutlookNode));
    registry.register(Arc::new(ExcelOnlineNode));
    registry.register(Arc::new(SharepointNode));
    registry.register(Arc::new(DynamicsNode));
}
