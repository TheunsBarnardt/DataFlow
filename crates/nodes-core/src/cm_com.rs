//! CM.com messaging node — SMS / WhatsApp via Messages API.
//! Integrations are nodes + credentials, not hardcoded platform services.

use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct CmComNode {
    client: Option<reqwest::Client>,
}

impl CmComNode {
    fn http(&self) -> Result<reqwest::Client, NodeError> {
        if let Some(c) = &self.client {
            return Ok(c.clone());
        }
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NodeError::msg(e.to_string()))
    }
}

#[async_trait]
impl NodeExecutor for CmComNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "cmCom".into(),
            display_name: "CM.com".into(),
            category: "api".into(),
            description: "Send SMS or WhatsApp messages via CM.com (OTP, notifications)."
                .into(),
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
        if ctx.cancel.is_cancelled() {
            return Err(NodeError::Cancelled);
        }

        let cred = ctx
            .credentials
            .values()
            .find(|c| {
                c.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "cmComApi" || t == "cmCom")
                    .unwrap_or(false)
                    || c.get("productToken").is_some()
                    || c.get("apiKey").is_some()
            })
            .cloned()
            .or_else(|| {
                // Credential payload may be nested under data / decrypted fields.
                ctx.credentials.values().next().cloned()
            });

        let product_token = cred
            .as_ref()
            .and_then(|c| {
                c.get("productToken")
                    .or_else(|| c.get("apiKey"))
                    .or_else(|| c.pointer("/data/productToken"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                NodeError::Config(
                    "CM.com credential with productToken (or apiKey) is required".into(),
                )
            })?;

        let operation = ctx
            .node
            .parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("sendSms");

        let from = ctx
            .node
            .parameters
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or("BotFlow");

        let to = ctx
            .node
            .parameters
            .get("to")
            .and_then(|v| v.as_str())
            .or_else(|| {
                ctx.input_items
                    .first()
                    .and_then(|i| i.get("to").or_else(|| i.get("phone")).and_then(|v| v.as_str()))
            })
            .ok_or_else(|| NodeError::Config("to (E.164 phone) is required".into()))?;

        let message = ctx
            .node
            .parameters
            .get("message")
            .and_then(|v| v.as_str())
            .or_else(|| {
                ctx.input_items
                    .first()
                    .and_then(|i| {
                        i.get("message")
                            .or_else(|| i.get("body"))
                            .or_else(|| i.get("otp"))
                            .and_then(|v| v.as_str())
                    })
            })
            .ok_or_else(|| NodeError::Config("message is required".into()))?;

        let channel = match operation {
            "sendWhatsApp" | "whatsapp" => "WhatsApp",
            _ => "SMS",
        };

        let reference = ctx
            .node
            .parameters
            .get("reference")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.run_id);

        let body = json!({
            "messages": {
                "authentication": { "productToken": product_token },
                "msg": [{
                    "from": from,
                    "to": [{ "number": to }],
                    "body": { "type": "auto", "content": message },
                    "allowedChannels": [channel],
                    "reference": reference,
                }]
            }
        });

        let url = ctx
            .node
            .parameters
            .get("apiUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("https://gw.cmtelecom.com/v1.0/message");

        let client = self.http()?;
        let res = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;

        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        let parsed: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));

        if !(200..300).contains(&status) {
            return Err(NodeError::msg(format!(
                "CM.com HTTP {status}: {text}"
            )));
        }

        let out = json!({
            "ok": true,
            "channel": channel,
            "to": to,
            "status": status,
            "response": parsed,
        });

        let mut items = ctx.input_items.clone();
        if items.is_empty() {
            items.push(out);
        } else if let Some(first) = items.first_mut() {
            if let Some(obj) = first.as_object_mut() {
                obj.insert("cmCom".into(), out);
            }
        }
        Ok(NodeResult::single(items))
    }
}

pub fn register(registry: &mut botflow_node_api::NodeRegistry) {
    use std::sync::Arc;
    registry.register(Arc::new(CmComNode::default()));
}
