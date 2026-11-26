use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::{json, Value};

pub struct SetNode;

#[async_trait]
impl NodeExecutor for SetNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "set".into(),
            display_name: "Set / Transform".into(),
            category: "data".into(),
            description: "Create or reshape JSON fields".into(),
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
        let assign = ctx
            .node
            .parameters
            .get("values")
            .cloned()
            .or_else(|| ctx.node.parameters.get("json").cloned())
            .unwrap_or_else(|| json!({}));

        let keep = ctx
            .node
            .parameters
            .get("keepOnlySet")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut out = Vec::new();
        for item in ctx.input_items {
            let mut base = if keep {
                json!({})
            } else {
                item
            };
            if let (Value::Object(dst), Value::Object(src)) = (&mut base, &assign) {
                for (k, v) in src {
                    dst.insert(k.clone(), v.clone());
                }
            } else if keep {
                base = assign.clone();
            }
            out.push(base);
        }
        if out.is_empty() {
            out.push(assign);
        }
        Ok(NodeResult::single(out))
    }
}

pub struct IfNode;

#[async_trait]
impl NodeExecutor for IfNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "if".into(),
            display_name: "IF".into(),
            category: "flow".into(),
            description: "Branch on a condition".into(),
            version: 1,
            is_trigger: false,
            inputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
            outputs: vec![
                PortDef {
                    id: "true".into(),
                    name: "True".into(),
                },
                PortDef {
                    id: "false".into(),
                    name: "False".into(),
                },
            ],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let mut tru = Vec::new();
        let mut fal = Vec::new();
        for item in ctx.input_items {
            if eval_condition(&ctx.node.parameters, &item) {
                tru.push(item);
            } else {
                fal.push(item);
            }
        }
        // Prefer emitting the taken branch with items; empty other branch omitted
        if !tru.is_empty() {
            Ok(NodeResult::branched("true", tru))
        } else {
            Ok(NodeResult::branched("false", fal))
        }
    }
}

fn eval_condition(params: &Value, item: &Value) -> bool {
    if let Some(b) = params.get("value").and_then(|v| v.as_bool()) {
        return b;
    }
    let left = resolve_path(
        item,
        params
            .get("field")
            .or_else(|| params.get("leftValue"))
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let op = params
        .get("operator")
        .or_else(|| params.get("op"))
        .and_then(|v| v.as_str())
        .unwrap_or("equals");
    let right = params
        .get("rightValue")
        .or_else(|| params.get("value"))
        .cloned()
        .unwrap_or(Value::Null);

    match op {
        "equals" | "eq" => left == right || left.as_str().map(|s| Value::String(s.into())) == Some(right),
        "notEquals" | "neq" => left != right,
        "exists" => !left.is_null(),
        "truthy" => match left {
            Value::Bool(b) => b,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::Null => false,
            _ => true,
        },
        _ => false,
    }
}

fn resolve_path(item: &Value, path: &str) -> Value {
    if path.is_empty() {
        return item.clone();
    }
    let mut cur = item;
    for part in path.split('.') {
        match cur {
            Value::Object(map) => {
                if let Some(v) = map.get(part) {
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

pub struct MergeNode;

#[async_trait]
impl NodeExecutor for MergeNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "merge".into(),
            display_name: "Merge".into(),
            category: "flow".into(),
            description: "Combine branch outputs".into(),
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
        Ok(NodeResult::single(ctx.input_items))
    }
}
