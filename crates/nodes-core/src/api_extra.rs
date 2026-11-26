use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use botflow_security::JwtAuth;
use serde_json::{json, Value};
use std::sync::Arc;

fn meta(ty: &str, name: &str, description: &str) -> NodeMetadata {
    NodeMetadata {
        type_name: ty.into(),
        display_name: name.into(),
        category: "api".into(),
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

pub struct WebhookRespondNode;
#[async_trait]
impl NodeExecutor for WebhookRespondNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "webhookRespond",
            "Respond to Webhook",
            "Send an HTTP response back to a webhook caller",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let status = ctx
            .node
            .parameters
            .get("statusCode")
            .and_then(|v| v.as_u64())
            .unwrap_or(200);
        let body = ctx
            .node
            .parameters
            .get("body")
            .cloned()
            .unwrap_or(json!({ "ok": true }));
        Ok(NodeResult::single(vec![json!({
            "respond": true,
            "statusCode": status,
            "body": body,
            "items": ctx.input_items,
        })]))
    }
}

pub struct ApiRespondNode;
#[async_trait]
impl NodeExecutor for ApiRespondNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "apiRespond",
            "Respond to API",
            "Return a response for an API endpoint request",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let status = ctx
            .node
            .parameters
            .get("statusCode")
            .and_then(|v| v.as_u64())
            .unwrap_or(200);
        let mode = ctx
            .node
            .parameters
            .get("responseData")
            .and_then(|v| v.as_str())
            .unwrap_or("firstEntryJson");
        let body = ctx
            .node
            .parameters
            .get("body")
            .cloned()
            .unwrap_or_else(|| match mode {
                "allEntries" => Value::Array(ctx.input_items.clone()),
                _ => ctx
                    .input_items
                    .first()
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            });
        Ok(NodeResult::single(vec![json!({
            "respond": true,
            "statusCode": status,
            "body": body,
        })]))
    }
}

pub struct GraphqlNode;
#[async_trait]
impl NodeExecutor for GraphqlNode {
    fn metadata(&self) -> NodeMetadata {
        meta("graphql", "GraphQL", "Run a GraphQL query or mutation")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let url = ctx
            .node
            .parameters
            .get("url")
            .or_else(|| ctx.node.parameters.get("endpoint"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("url required".into()))?;
        let query = ctx
            .node
            .parameters
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("{ __typename }");
        let variables = ctx
            .node
            .parameters
            .get("variables")
            .cloned()
            .unwrap_or(json!({}));
        let client = reqwest::Client::new();
        let mut req = client.post(url).json(&json!({ "query": query, "variables": variables }));
        if let Ok(token) = std::env::var("GRAPHQL_TOKEN") {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        let body: serde_json::Value = resp.json().await.map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![body]))
    }
}

pub struct JwtNode;
#[async_trait]
impl NodeExecutor for JwtNode {
    fn metadata(&self) -> NodeMetadata {
        meta("jwt", "JWT", "Sign or verify JSON Web Tokens")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let op = ctx
            .node
            .parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("sign");
        let secret = ctx
            .node
            .parameters
            .get("secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("BOTFLOW_JWT_SECRET").ok())
            .unwrap_or_else(|| "botflow-dev-jwt-secret".into());
        let auth = JwtAuth::new(&secret, 24);
        match op {
            "verify" => {
                let token = ctx
                    .node
                    .parameters
                    .get("token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| NodeError::Config("token required".into()))?;
                let claims = auth
                    .verify(token)
                    .map_err(|e| NodeError::msg(e.to_string()))?;
                Ok(NodeResult::single(vec![json!({
                    "sub": claims.sub,
                    "role": claims.role,
                    "exp": claims.exp,
                })]))
            }
            _ => {
                let sub = ctx
                    .node
                    .parameters
                    .get("sub")
                    .and_then(|v| v.as_str())
                    .unwrap_or("botflow");
                let role = ctx
                    .node
                    .parameters
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");
                let token = auth
                    .issue(sub, role)
                    .map_err(|e| NodeError::msg(e.to_string()))?;
                Ok(NodeResult::single(vec![json!({ "token": token })]))
            }
        }
    }
}

pub struct RssReadNode;
#[async_trait]
impl NodeExecutor for RssReadNode {
    fn metadata(&self) -> NodeMetadata {
        meta("rssRead", "RSS Read", "Fetch and read an RSS or Atom feed")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let url = ctx
            .node
            .parameters
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("url required".into()))?;
        let text = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?
            .text()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "feedXml": text })]))
    }
}

macro_rules! stub_api {
    ($struct:ident, $ty:expr, $name:expr, $desc:expr) => {
        pub struct $struct;
        #[async_trait]
        impl NodeExecutor for $struct {
            fn metadata(&self) -> NodeMetadata {
                meta($ty, $name, $desc)
            }
            async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
                Ok(NodeResult::single(vec![json!({
                    "node": $ty,
                    "parameters": ctx.node.parameters,
                    "items": ctx.input_items,
                })]))
            }
        }
    };
}

stub_api!(N8nApiNode, "n8nApi", "n8n", "Call the n8n public API");
stub_api!(FormNode, "form", "Form", "Collect or process form field values");
stub_api!(FtpNode, "ftp", "FTP", "Upload or download files over FTP/SFTP");
stub_api!(SshNode, "ssh", "SSH", "Run a command on a remote host over SSH");
stub_api!(GitNode, "git", "Git", "Run Git operations on a repository");
stub_api!(LdapNode, "ldap", "LDAP", "Search or update directory entries via LDAP");

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(WebhookRespondNode));
    registry.register(Arc::new(ApiRespondNode));
    registry.register(Arc::new(GraphqlNode));
    registry.register(Arc::new(JwtNode));
    registry.register(Arc::new(RssReadNode));
    registry.register(Arc::new(N8nApiNode));
    registry.register(Arc::new(FormNode));
    registry.register(Arc::new(FtpNode));
    registry.register(Arc::new(SshNode));
    registry.register(Arc::new(GitNode));
    registry.register(Arc::new(LdapNode));
}
