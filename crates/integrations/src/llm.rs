use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Clone, Default)]
pub struct LlmNode;

#[async_trait]
impl NodeExecutor for LlmNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "llm".into(),
            display_name: "LLM".into(),
            category: "ai".into(),
            description: "Provider-based LLM call with budget/allowlist controls".into(),
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
        let prompt = ctx
            .node
            .parameters
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let model = ctx
            .node
            .parameters
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("OPENAI_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o-mini".into());

        let allowlist: HashSet<String> = std::env::var("LLM_MODEL_ALLOWLIST")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();
        if !allowlist.is_empty() && !allowlist.contains(&model) {
            return Err(NodeError::msg(format!("model '{model}' not in LLM_MODEL_ALLOWLIST")));
        }

        let max_tokens = ctx
            .node
            .parameters
            .get("maxTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(512);
        if max_tokens > 4096 {
            return Err(NodeError::msg("maxTokens exceeds budget cap 4096"));
        }

        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            // Deterministic offline stub for tests / local without key
            return Ok(NodeResult::single(vec![json!({
                "model": model,
                "stub": true,
                "output": format!("[stub] {}", prompt),
            })]));
        }

        let base = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        let client = reqwest::Client::new();
        let body = json!({
            "model": model,
            "messages": [{"role":"user","content": prompt}],
            "max_tokens": max_tokens,
        });
        let resp = client
            .post(format!("{base}/chat/completions"))
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        let status = resp.status();
        let v: Value = resp.json().await.map_err(|e| NodeError::msg(e.to_string()))?;
        if !status.is_success() {
            return Err(NodeError::msg(format!("llm error: {v}")));
        }
        let content = v["choices"][0]["message"]["content"].clone();
        Ok(NodeResult::single(vec![json!({
            "model": model,
            "output": content,
            "raw": v,
        })]))
    }
}

pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "approval".into(),
            display_name: "Human Approval".into(),
            category: "ai".into(),
            description: "Pause for human approval before continuing".into(),
            version: 1,
            is_trigger: false,
            inputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
            outputs: vec![
                PortDef {
                    id: "approved".into(),
                    name: "Approved".into(),
                },
                PortDef {
                    id: "rejected".into(),
                    name: "Rejected".into(),
                },
            ],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        // Auto-approve when APPROVAL_AUTO=1 (dev); otherwise require parameters.decision
        let decision = ctx
            .node
            .parameters
            .get("decision")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                if std::env::var("APPROVAL_AUTO").ok().as_deref() == Some("1") {
                    Some("approved".into())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                NodeError::msg("approval required: set parameters.decision or APPROVAL_AUTO=1")
            })?;
        let handle = if decision == "approved" {
            "approved"
        } else {
            "rejected"
        };
        Ok(NodeResult::branched(handle, ctx.input_items))
    }
}
