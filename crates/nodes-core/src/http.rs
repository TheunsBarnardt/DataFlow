use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use botflow_security::SsrfGuard;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone)]
pub struct HttpRequestNode {
    client: reqwest::Client,
    ssrf: SsrfGuard,
}

impl Default for HttpRequestNode {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client"),
            ssrf: SsrfGuard {
                allow_private: std::env::var("HTTP_ALLOW_PRIVATE")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(true), // local-first default; tighten in prod
                blocked_hosts: Default::default(),
            },
        }
    }
}

#[async_trait]
impl NodeExecutor for HttpRequestNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "httpRequest".into(),
            display_name: "HTTP Request".into(),
            category: "api".into(),
            description: "Call REST services".into(),
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
        let url = ctx
            .node
            .parameters
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("url is required".into()))?;
        self.ssrf
            .validate_url(url)
            .map_err(|e| NodeError::msg(e.to_string()))?;

        let method = ctx
            .node
            .parameters
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let mut req = match method.as_str() {
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "PATCH" => self.client.patch(url),
            "DELETE" => self.client.delete(url),
            _ => self.client.get(url),
        };

        if let Some(headers) = ctx.node.parameters.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in headers {
                if let Some(s) = v.as_str() {
                    req = req.header(k, s);
                }
            }
        }

        if let Some(body) = ctx.node.parameters.get("body") {
            req = req.json(body);
        }

        let retries = ctx
            .node
            .settings
            .as_ref()
            .and_then(|s| s.max_tries)
            .unwrap_or(1)
            .max(1);
        let wait_ms = ctx
            .node
            .settings
            .as_ref()
            .and_then(|s| s.wait_between_tries)
            .unwrap_or(500);

        let mut last_err = None;
        for attempt in 0..retries {
            if ctx.cancel.is_cancelled() {
                return Err(NodeError::Cancelled);
            }
            match req.try_clone().unwrap_or_else(|| self.client.get(url)).send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let headers: Value = resp
                        .headers()
                        .iter()
                        .filter_map(|(k, v)| {
                            v.to_str()
                                .ok()
                                .map(|s| (k.to_string(), Value::String(s.to_string())))
                        })
                        .collect::<serde_json::Map<_, _>>()
                        .into();
                    let body_text = resp.text().await.unwrap_or_default();
                    let body_json: Value =
                        serde_json::from_str(&body_text).unwrap_or(Value::String(body_text));
                    let item = json!({
                        "statusCode": status,
                        "headers": headers,
                        "body": body_json,
                    });
                    return Ok(NodeResult::single(vec![item]));
                }
                Err(e) => {
                    last_err = Some(e.to_string());
                    if attempt + 1 < retries {
                        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                    }
                }
            }
        }
        Err(NodeError::msg(last_err.unwrap_or_else(|| "http failed".into())))
    }
}
