//! Node parameter schemas (exported from web/src/lib/nodeSchemas.ts).

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub options: Option<Vec<FieldOption>>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub when: Option<WhenClause>,
    #[serde(default)]
    pub credential_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WhenClause {
    pub key: String,
    pub equals: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeSchema {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub defaults: HashMap<String, Value>,
}

static SCHEMAS: OnceLock<HashMap<String, NodeSchema>> = OnceLock::new();

fn load() -> &'static HashMap<String, NodeSchema> {
    SCHEMAS.get_or_init(|| {
        let raw = include_str!("../../assets/node-schemas.json");
        serde_json::from_str(raw).unwrap_or_default()
    })
}

pub fn get_schema(node_type: &str) -> Option<&'static NodeSchema> {
    load().get(node_type)
}

pub fn default_parameters(node_type: &str) -> HashMap<String, Value> {
    get_schema(node_type)
        .map(|s| s.defaults.clone())
        .unwrap_or_default()
}

pub fn field_visible(field: &FieldDef, parameters: &Value) -> bool {
    let Some(when) = &field.when else {
        return true;
    };
    let current = parameters.get(&when.key);
    match &when.equals {
        Value::String(s) => current.and_then(|v| v.as_str()) == Some(s.as_str()),
        Value::Array(arr) => arr.iter().any(|eq| current == Some(eq)),
        other => current == Some(other),
    }
}

pub fn compare_ops() -> &'static [FieldOption] {
    static OPS: OnceLock<Vec<FieldOption>> = OnceLock::new();
    OPS.get_or_init(|| {
        [
            ("equals", "Equals"),
            ("notEquals", "Not equals"),
            ("contains", "Contains"),
            ("notContains", "Not contains"),
            ("startsWith", "Starts with"),
            ("endsWith", "Ends with"),
            ("regex", "Regex"),
            ("exists", "Exists"),
            ("notExists", "Does not exist"),
            ("empty", "Is empty"),
            ("notEmpty", "Is not empty"),
            ("truthy", "Truthy"),
            ("falsy", "Falsy"),
            ("gt", "Greater than"),
            ("gte", "Greater or equal"),
            ("lt", "Less than"),
            ("lte", "Less or equal"),
        ]
        .into_iter()
        .map(|(value, label)| FieldOption {
            value: value.into(),
            label: label.into(),
        })
        .collect()
    })
}

pub fn is_credential_field(key: &str) -> bool {
    let k = key.to_lowercase();
    k == "credentialid" || k.contains("credential")
}

pub fn merge_defaults(node_type: &str, existing: &Value) -> Value {
    let mut out = Value::Object(serde_json::Map::new());
    if let Some(obj) = out.as_object_mut() {
        for (k, v) in default_parameters(node_type) {
            obj.insert(k, v);
        }
        if let Some(map) = existing.as_object() {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    out
}

/// Drop hidden fields and coerce json/number kinds (mirrors React `sanitizeParameters`).
pub fn sanitize_parameters(
    schema: &NodeSchema,
    values: &Value,
) -> Result<Value, String> {
    let mut out = values.clone();
    let Some(obj) = out.as_object_mut() else {
        return Ok(out);
    };
    for field in &schema.fields {
        if !field_visible(field, values) {
            obj.remove(&field.key);
            continue;
        }
        if field.kind == "json" || field.kind == "jsonSchema" {
            match obj.get(&field.key) {
                Some(Value::String(s)) => {
                    let raw = s.trim();
                    if raw.is_empty() {
                        obj.remove(&field.key);
                    } else {
                        match serde_json::from_str::<Value>(raw) {
                            Ok(parsed) => {
                                obj.insert(field.key.clone(), parsed);
                            }
                            Err(_) => {
                                return Err(format!("Invalid JSON in {}", field.label));
                            }
                        }
                    }
                }
                Some(Value::Null) => {
                    obj.remove(&field.key);
                }
                _ => {}
            }
        }
        if field.kind == "number" {
            if let Some(v) = obj.get(&field.key) {
                if let Some(n) = v.as_f64() {
                    if !n.is_finite() {
                        return Err(format!("{} must be a number", field.label));
                    }
                } else if let Some(s) = v.as_str() {
                    match s.trim().parse::<f64>() {
                        Ok(n) if n.is_finite() => {
                            obj.insert(field.key.clone(), Value::from(n));
                        }
                        _ => {
                            return Err(format!("{} must be a number", field.label));
                        }
                    }
                } else if !v.is_null() {
                    return Err(format!("{} must be a number", field.label));
                }
            }
        }
    }
    Ok(out)
}
