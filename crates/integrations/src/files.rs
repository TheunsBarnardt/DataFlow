use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::json;
use std::path::PathBuf;

fn resolve_safe(path: &str) -> Result<PathBuf, NodeError> {
    let base = std::env::var("BOTFLOW_FILE_ROOT").unwrap_or_else(|_| ".".into());
    let root = PathBuf::from(&base)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(base));
    let joined = root.join(path);
    let canon = joined
        .canonicalize()
        .unwrap_or(joined.clone());
    if !canon.starts_with(&root) && path.contains("..") {
        return Err(NodeError::Config("path escapes BOTFLOW_FILE_ROOT".into()));
    }
    Ok(joined)
}

pub struct ReadFileNode;

#[async_trait]
impl NodeExecutor for ReadFileNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "readFile".into(),
            display_name: "Read File".into(),
            category: "files".into(),
            description: "Read a file from disk".into(),
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
        let path = ctx
            .node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("path is required".into()))?;
        let p = resolve_safe(path)?;
        let data = tokio::fs::read_to_string(&p)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "path": path, "content": data })]))
    }
}

pub struct WriteFileNode;

#[async_trait]
impl NodeExecutor for WriteFileNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "writeFile".into(),
            display_name: "Write File".into(),
            category: "files".into(),
            description: "Write a file to disk".into(),
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
        let path = ctx
            .node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("path is required".into()))?;
        let content = ctx
            .node
            .parameters
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let p = resolve_safe(path)?;
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| NodeError::msg(e.to_string()))?;
        }
        tokio::fs::write(&p, content)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "path": path, "written": true })]))
    }
}
