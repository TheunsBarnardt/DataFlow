use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn meta(ty: &str, name: &str, description: &str) -> NodeMetadata {
    NodeMetadata {
        type_name: ty.into(),
        display_name: name.into(),
        category: "files".into(),
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

fn root() -> PathBuf {
    let base = std::env::var("BOTFLOW_FILE_ROOT").unwrap_or_else(|_| ".".into());
    PathBuf::from(base)
}

fn resolve(path: &str) -> Result<PathBuf, NodeError> {
    let joined = root().join(path);
    if path.contains("..") {
        return Err(NodeError::Config("path escapes root".into()));
    }
    Ok(joined)
}

pub struct ListFilesNode;
#[async_trait]
impl NodeExecutor for ListFilesNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "listFiles",
            "List Files",
            "List files and folders in a directory",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let path = ctx
            .node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let p = resolve(path)?;
        let mut entries = Vec::new();
        let mut rd = tokio::fs::read_dir(&p)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        while let Some(e) = rd
            .next_entry()
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?
        {
            entries.push(json!({
                "name": e.file_name().to_string_lossy(),
                "path": e.path().to_string_lossy(),
                "isDir": e.file_type().await.map(|t| t.is_dir()).unwrap_or(false),
            }));
        }
        Ok(NodeResult::single(entries))
    }
}

pub struct MoveFileNode;
#[async_trait]
impl NodeExecutor for MoveFileNode {
    fn metadata(&self) -> NodeMetadata {
        meta("moveFile", "Move File", "Move or rename a file on disk")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let from = ctx
            .node
            .parameters
            .get("from")
            .or_else(|| ctx.node.parameters.get("sourcePath"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("from required".into()))?;
        let to = ctx
            .node
            .parameters
            .get("to")
            .or_else(|| ctx.node.parameters.get("destinationPath"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("to required".into()))?;
        let a = resolve(from)?;
        let b = resolve(to)?;
        tokio::fs::rename(&a, &b)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "from": from, "to": to })]))
    }
}

pub struct CopyFileNode;
#[async_trait]
impl NodeExecutor for CopyFileNode {
    fn metadata(&self) -> NodeMetadata {
        meta("copyFile", "Copy File", "Copy a file to another path")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let from = ctx
            .node
            .parameters
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("from required".into()))?;
        let to = ctx
            .node
            .parameters
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("to required".into()))?;
        tokio::fs::copy(resolve(from)?, resolve(to)?)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        Ok(NodeResult::single(vec![json!({ "from": from, "to": to })]))
    }
}

pub struct DeleteFileNode;
#[async_trait]
impl NodeExecutor for DeleteFileNode {
    fn metadata(&self) -> NodeMetadata {
        meta(
            "deleteFile",
            "Delete File",
            "Delete a file or folder from disk",
        )
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let path = ctx
            .node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("path required".into()))?;
        let p = resolve(path)?;
        if p.is_dir() {
            tokio::fs::remove_dir_all(&p)
                .await
                .map_err(|e| NodeError::msg(e.to_string()))?;
        } else {
            tokio::fs::remove_file(&p)
                .await
                .map_err(|e| NodeError::msg(e.to_string()))?;
        }
        Ok(NodeResult::single(vec![json!({ "deleted": path })]))
    }
}

pub struct JsonNode;
#[async_trait]
impl NodeExecutor for JsonNode {
    fn metadata(&self) -> NodeMetadata {
        meta("json", "JSON", "Parse or stringify JSON data")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let op = ctx
            .node
            .parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("parse");
        let mut out = Vec::new();
        for item in ctx.input_items {
            match op {
                "stringify" => out.push(json!({ "json": item.to_string() })),
                _ => {
                    if let Some(s) = item.as_str().or_else(|| {
                        item.get("content")
                            .and_then(|v| v.as_str())
                    }) {
                        let parsed: serde_json::Value =
                            serde_json::from_str(s).unwrap_or(json!({ "raw": s }));
                        out.push(parsed);
                    } else {
                        out.push(item);
                    }
                }
            }
        }
        Ok(NodeResult::single(out))
    }
}

pub struct CsvNode;
#[async_trait]
impl NodeExecutor for CsvNode {
    fn metadata(&self) -> NodeMetadata {
        meta("csv", "CSV", "Parse CSV text into rows of objects")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let text = ctx
            .node
            .parameters
            .get("csv")
            .or_else(|| ctx.node.parameters.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mut lines = text.lines();
        let headers: Vec<String> = lines
            .next()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut rows = Vec::new();
        for line in lines {
            let cols: Vec<&str> = line.split(',').collect();
            let mut obj = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                obj.insert(
                    h.clone(),
                    json!(cols.get(i).copied().unwrap_or("").trim()),
                );
            }
            rows.push(serde_json::Value::Object(obj));
        }
        Ok(NodeResult::single(rows))
    }
}

pub struct XmlNode;
#[async_trait]
impl NodeExecutor for XmlNode {
    fn metadata(&self) -> NodeMetadata {
        meta("xml", "XML", "Pass through or wrap XML content")
    }
    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let xml = ctx
            .node
            .parameters
            .get("xml")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(NodeResult::single(vec![json!({ "xml": xml })]))
    }
}

macro_rules! stub_file {
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

stub_file!(
    ExcelNode,
    "excel",
    "Excel File",
    "Read or write Excel spreadsheet files"
);
stub_file!(ZipNode, "zip", "Zip", "Create or extract ZIP archives");
stub_file!(
    CompressionNode,
    "compression",
    "Compression",
    "Compress or decompress file data"
);
stub_file!(
    ConvertToFileNode,
    "convertToFile",
    "Convert to File",
    "Convert item data into a downloadable file"
);
stub_file!(
    ExtractFromFileNode,
    "extractFromFile",
    "Extract from File",
    "Extract content and fields from a file"
);
stub_file!(
    ReadPdfNode,
    "readPdf",
    "Read PDF",
    "Extract text and metadata from a PDF"
);
stub_file!(
    MoveBinaryDataNode,
    "moveBinaryData",
    "Move Binary Data",
    "Move binary data between items or fields"
);
stub_file!(
    EditImageNode,
    "editImage",
    "Edit Image",
    "Resize, crop, or transform an image"
);
stub_file!(
    IcalendarNode,
    "icalendar",
    "iCalendar",
    "Parse or create iCalendar (.ics) events"
);

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(ListFilesNode));
    registry.register(Arc::new(MoveFileNode));
    registry.register(Arc::new(CopyFileNode));
    registry.register(Arc::new(DeleteFileNode));
    registry.register(Arc::new(JsonNode));
    registry.register(Arc::new(CsvNode));
    registry.register(Arc::new(XmlNode));
    registry.register(Arc::new(ExcelNode));
    registry.register(Arc::new(ZipNode));
    registry.register(Arc::new(CompressionNode));
    registry.register(Arc::new(ConvertToFileNode));
    registry.register(Arc::new(ExtractFromFileNode));
    registry.register(Arc::new(ReadPdfNode));
    registry.register(Arc::new(MoveBinaryDataNode));
    registry.register(Arc::new(EditImageNode));
    registry.register(Arc::new(IcalendarNode));
    let _ = Path::new(".");
}
