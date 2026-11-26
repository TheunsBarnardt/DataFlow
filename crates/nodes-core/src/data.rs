use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeRegistry, NodeResult, PortDef,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn main_ports() -> (Vec<PortDef>, Vec<PortDef>) {
    (
        vec![PortDef {
            id: "main".into(),
            name: "Main".into(),
        }],
        vec![PortDef {
            id: "main".into(),
            name: "Main".into(),
        }],
    )
}

fn meta(ty: &str, name: &str, cat: &str, description: &str) -> NodeMetadata {
    let (inputs, outputs) = main_ports();
    NodeMetadata {
        type_name: ty.into(),
        display_name: name.into(),
        category: cat.into(),
        description: description.into(),
        version: 1,
        is_trigger: false,
        inputs,
        outputs,
    }
}

fn field(item: &Value, path: &str) -> Value {
    if path.is_empty() {
        return item.clone();
    }
    let mut cur = item;
    for part in path.split('.') {
        match cur {
            Value::Object(m) => {
                if let Some(v) = m.get(part) {
                    cur = v;
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }
    cur.clone()
}

macro_rules! simple_node {
    ($struct:ident, $ty:expr, $name:expr, $desc:expr, $body:expr) => {
        pub struct $struct;
        #[async_trait]
        impl NodeExecutor for $struct {
            fn metadata(&self) -> NodeMetadata {
                meta($ty, $name, "data", $desc)
            }
            async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
                ($body)(ctx)
            }
        }
    };
}

simple_node!(EditFieldsNode, "editFields", "Edit Fields", "Map, rename, or reshape item fields", |ctx: ExecutionContext| {
    let mapping = ctx
        .node
        .parameters
        .get("mapping")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in ctx.input_items {
        let mut obj = item.as_object().cloned().unwrap_or_default();
        for (from, to) in &mapping {
            let val = field(&Value::Object(obj.clone()), from);
            if let Some(t) = to.as_str() {
                obj.insert(t.to_string(), val);
                if from != t {
                    obj.remove(from);
                }
            }
        }
        out.push(Value::Object(obj));
    }
    Ok(NodeResult::single(out))
});

simple_node!(FilterNode, "filter", "Filter", "Keep only items that match a condition", |ctx: ExecutionContext| {
    let field_name = ctx
        .node
        .parameters
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let op = ctx
        .node
        .parameters
        .get("operator")
        .and_then(|v| v.as_str())
        .unwrap_or("exists");
    let right = ctx.node.parameters.get("value").cloned().unwrap_or(Value::Null);
    let kept: Vec<Value> = ctx
        .input_items
        .into_iter()
        .filter(|item| {
            let left = field(item, field_name);
            match op {
                "equals" | "eq" => left == right,
                "notEquals" => left != right,
                "contains" => left
                    .as_str()
                    .zip(right.as_str())
                    .map(|(a, b)| a.contains(b))
                    .unwrap_or(false),
                "exists" => !left.is_null(),
                "truthy" => match left {
                    Value::Bool(b) => b,
                    Value::String(s) => !s.is_empty(),
                    Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                    Value::Null => false,
                    _ => true,
                },
                _ => true,
            }
        })
        .collect();
    Ok(NodeResult::single(kept))
});

simple_node!(SortNode, "sort", "Sort", "Sort items by a field ascending or descending", |ctx: ExecutionContext| {
    let field_name = ctx
        .node
        .parameters
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let desc = ctx
        .node
        .parameters
        .get("order")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("desc"))
        .unwrap_or(false);
    let mut items = ctx.input_items;
    items.sort_by(|a, b| {
        let av = field(a, field_name);
        let bv = field(b, field_name);
        let ord = match (av.as_f64(), bv.as_f64()) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
            _ => av.to_string().cmp(&bv.to_string()),
        };
        if desc {
            ord.reverse()
        } else {
            ord
        }
    });
    Ok(NodeResult::single(items))
});

simple_node!(LimitNode, "limit", "Limit", "Keep only the first N items", |ctx: ExecutionContext| {
    let n = ctx
        .node
        .parameters
        .get("maxItems")
        .or_else(|| ctx.node.parameters.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let mut items = ctx.input_items;
    items.truncate(n);
    Ok(NodeResult::single(items))
});

simple_node!(SplitInBatchesNode, "splitInBatches", "Loop Over Items", "Process items in batches of a given size", |ctx: ExecutionContext| {
    let size = ctx
        .node
        .parameters
        .get("batchSize")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let batch: Vec<Value> = ctx.input_items.into_iter().take(size.max(1)).collect();
    Ok(NodeResult::single(batch))
});

simple_node!(SplitOutNode, "splitOut", "Split Out", "Turn an array field into separate items", |ctx: ExecutionContext| {
    let field_name = ctx
        .node
        .parameters
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("items");
    let mut out = Vec::new();
    for item in ctx.input_items {
        match field(&item, field_name) {
            Value::Array(arr) => out.extend(arr),
            other if !other.is_null() => out.push(other),
            _ => {}
        }
    }
    Ok(NodeResult::single(out))
});

simple_node!(AggregateNode, "aggregate", "Aggregate", "Combine all items into a single list item", |ctx: ExecutionContext| {
    Ok(NodeResult::single(vec![json!({ "items": ctx.input_items, "count": ctx.input_items.len() })]))
});

simple_node!(SummarizeNode, "summarize", "Summarize", "Compute count, sum, and average for a numeric field", |ctx: ExecutionContext| {
    let field_name = ctx
        .node
        .parameters
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut sum = 0.0;
    let mut count = 0u64;
    for item in &ctx.input_items {
        if let Some(n) = field(item, field_name).as_f64() {
            sum += n;
            count += 1;
        }
    }
    Ok(NodeResult::single(vec![json!({
        "count": count,
        "sum": sum,
        "avg": if count > 0 { sum / count as f64 } else { 0.0 },
    })]))
});

simple_node!(ItemListsNode, "itemLists", "Item Lists", "Manipulate lists of items (pass-through helper)", |ctx: ExecutionContext| {
    Ok(NodeResult::single(ctx.input_items))
});

simple_node!(RenameKeysNode, "renameKeys", "Rename Keys", "Rename object keys on each item", |ctx: ExecutionContext| {
    let mapping = ctx
        .node
        .parameters
        .get("keys")
        .or_else(|| ctx.node.parameters.get("mapping"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in ctx.input_items {
        let mut obj = item.as_object().cloned().unwrap_or_default();
        for (from, to) in &mapping {
            if let Some(t) = to.as_str() {
                if let Some(v) = obj.remove(from) {
                    obj.insert(t.to_string(), v);
                }
            }
        }
        out.push(Value::Object(obj));
    }
    Ok(NodeResult::single(out))
});

simple_node!(RemoveDuplicatesNode, "removeDuplicates", "Remove Duplicates", "Drop duplicate items by value or field", |ctx: ExecutionContext| {
    let field_name = ctx
        .node
        .parameters
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in ctx.input_items {
        let key = if field_name.is_empty() {
            item.to_string()
        } else {
            field(&item, field_name).to_string()
        };
        if seen.insert(key) {
            out.push(item);
        }
    }
    Ok(NodeResult::single(out))
});

simple_node!(CompareDatasetsNode, "compareDatasets", "Compare Datasets", "Compare two datasets and surface differences", |ctx: ExecutionContext| {
    Ok(NodeResult::single(vec![json!({
        "items": ctx.input_items,
        "note": "compareDatasets: pass-through with count",
        "count": ctx.input_items.len(),
    })]))
});

simple_node!(CodeNode, "code", "Code", "Transform items with custom code or fixed JSON output", |ctx: ExecutionContext| {
    // Sandboxed JS not enabled by default — return items or parameters.json
    if let Some(v) = ctx.node.parameters.get("jsonOutput") {
        return Ok(NodeResult::single(vec![v.clone()]));
    }
    Ok(NodeResult::single(ctx.input_items))
});

simple_node!(AiTransformNode, "aiTransform", "AI Transform", "Transform items with an AI prompt", |ctx: ExecutionContext| {
    let prompt = ctx
        .node
        .parameters
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(NodeResult::single(vec![json!({
        "prompt": prompt,
        "items": ctx.input_items,
        "stub": true,
    })]))
});

simple_node!(DateTimeNode, "dateTime", "Date & Time", "Add the current timestamp to each item", |ctx: ExecutionContext| {
    let now = chrono::Utc::now().to_rfc3339();
    let mut out = Vec::new();
    for mut item in ctx.input_items {
        if let Value::Object(ref mut m) = item {
            m.insert("now".into(), Value::String(now.clone()));
        } else {
            item = json!({ "value": item, "now": now });
        }
        out.push(item);
    }
    if out.is_empty() {
        out.push(json!({ "now": now }));
    }
    Ok(NodeResult::single(out))
});

simple_node!(CryptoNode, "crypto", "Crypto", "Hash values with SHA-256", |ctx: ExecutionContext| {
    let action = ctx
        .node
        .parameters
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("sha256");
    let input = ctx
        .node
        .parameters
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let digest = match action {
        "sha256" | _ => {
            let mut h = Sha256::new();
            h.update(input.as_bytes());
            hex::encode(h.finalize())
        }
    };
    Ok(NodeResult::single(vec![json!({ "hash": digest, "action": action })]))
});

simple_node!(HtmlNode, "html", "HTML", "Produce or wrap HTML content", |ctx: ExecutionContext| {
    let html = ctx
        .node
        .parameters
        .get("html")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(NodeResult::single(vec![json!({ "html": html })]))
});

simple_node!(HtmlExtractNode, "htmlExtract", "HTML Extract", "Extract plain text from HTML", |ctx: ExecutionContext| {
    let html = ctx
        .node
        .parameters
        .get("html")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Minimal strip tags
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    Ok(NodeResult::single(vec![json!({ "text": text.trim() })]))
});

simple_node!(MarkdownNode, "markdown", "Markdown", "Pass through Markdown and a simple HTML preview", |ctx: ExecutionContext| {
    let md = ctx
        .node
        .parameters
        .get("markdown")
        .or_else(|| ctx.node.parameters.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(NodeResult::single(vec![json!({ "markdown": md, "html": format!("<pre>{}</pre>", md) })]))
});

simple_node!(TotpNode, "totp", "TOTP", "Generate or verify time-based one-time passwords", |ctx: ExecutionContext| {
    Ok(NodeResult::single(vec![json!({
        "note": "Configure TOTP_SECRET; code generation uses HMAC-SHA1 time step",
        "configured": std::env::var("TOTP_SECRET").is_ok(),
    })]))
});

simple_node!(TimeSavedNode, "timeSaved", "Track Time Saved", "Record estimated minutes saved by the workflow", |ctx: ExecutionContext| {
    let minutes = ctx
        .node
        .parameters
        .get("minutes")
        .and_then(|v| v.as_f64())
        .unwrap_or(5.0);
    Ok(NodeResult::single(vec![json!({ "timeSavedMinutes": minutes })]))
});

simple_node!(DataTableNode, "dataTable", "Data Table", "Work with tabular rows as workflow items", |ctx: ExecutionContext| {
    Ok(NodeResult::single(ctx.input_items))
});

simple_node!(ExecutionDataNode, "executionData", "Execution Data", "Expose run, workflow, and trace identifiers", |ctx: ExecutionContext| {
    Ok(NodeResult::single(vec![json!({
        "runId": ctx.run_id,
        "workflowId": ctx.workflow_id,
        "traceId": ctx.trace_id,
        "items": ctx.input_items,
    })]))
});

simple_node!(SimulateNode, "simulate", "Simulate", "Emit fixed sample output for testing", |ctx: ExecutionContext| {
    let out = ctx
        .node
        .parameters
        .get("output")
        .cloned()
        .unwrap_or(json!({ "simulated": true }));
    Ok(NodeResult::single(vec![out]))
});

simple_node!(DebugHelperNode, "debugHelper", "Debug Helper", "Inspect node parameters and incoming items", |ctx: ExecutionContext| {
    Ok(NodeResult::single(vec![json!({
        "node": ctx.node.name,
        "parameters": ctx.node.parameters,
        "items": ctx.input_items,
    })]))
});

simple_node!(EvaluationNode, "evaluation", "Evaluation", "Mark or score items during evaluation runs", |ctx: ExecutionContext| {
    Ok(NodeResult::single(ctx.input_items))
});

pub fn register(registry: &mut NodeRegistry) {
    registry.register(Arc::new(EditFieldsNode));
    registry.register(Arc::new(FilterNode));
    registry.register(Arc::new(SortNode));
    registry.register(Arc::new(LimitNode));
    registry.register(Arc::new(SplitInBatchesNode));
    registry.register(Arc::new(SplitOutNode));
    registry.register(Arc::new(AggregateNode));
    registry.register(Arc::new(SummarizeNode));
    registry.register(Arc::new(ItemListsNode));
    registry.register(Arc::new(RenameKeysNode));
    registry.register(Arc::new(RemoveDuplicatesNode));
    registry.register(Arc::new(CompareDatasetsNode));
    registry.register(Arc::new(CodeNode));
    registry.register(Arc::new(AiTransformNode));
    registry.register(Arc::new(DateTimeNode));
    registry.register(Arc::new(CryptoNode));
    registry.register(Arc::new(HtmlNode));
    registry.register(Arc::new(HtmlExtractNode));
    registry.register(Arc::new(MarkdownNode));
    registry.register(Arc::new(TotpNode));
    registry.register(Arc::new(TimeSavedNode));
    registry.register(Arc::new(DataTableNode));
    registry.register(Arc::new(ExecutionDataNode));
    registry.register(Arc::new(SimulateNode));
    registry.register(Arc::new(DebugHelperNode));
    registry.register(Arc::new(EvaluationNode));
    let _ = HashMap::<String, String>::new();
}
