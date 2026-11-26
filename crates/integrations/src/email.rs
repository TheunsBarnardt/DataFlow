use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde_json::json;

pub struct SendEmailNode;

#[async_trait]
impl NodeExecutor for SendEmailNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "sendEmail".into(),
            display_name: "Send Email".into(),
            category: "communication".into(),
            description: "Send email via SMTP".into(),
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
        let to = ctx
            .node
            .parameters
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("to is required".into()))?;
        let subject = ctx
            .node
            .parameters
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(no subject)");
        let body = ctx
            .node
            .parameters
            .get("text")
            .or_else(|| ctx.node.parameters.get("body"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let from = ctx
            .node
            .parameters
            .get("from")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SMTP_FROM").ok())
            .ok_or_else(|| NodeError::Config("from or SMTP_FROM required".into()))?;

        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);

        let email = Message::builder()
            .from(
                from.parse::<Mailbox>()
                    .map_err(|e| NodeError::Config(e.to_string()))?,
            )
            .to(to
                .parse::<Mailbox>()
                .map_err(|e| NodeError::Config(e.to_string()))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| NodeError::msg(e.to_string()))?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| NodeError::msg(e.to_string()))?
            .port(port);
        if let (Ok(user), Ok(pass)) = (std::env::var("SMTP_USER"), std::env::var("SMTP_PASSWORD")) {
            builder = builder.credentials(Credentials::new(user, pass));
        }
        let mailer = builder.build();
        mailer
            .send(email)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "sent": true, "to": to })]))
    }
}
