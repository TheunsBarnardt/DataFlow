//! JSON Schema helpers for API endpoint mapping UX (port of web/src/lib/jsonSchemaSample.ts).

use serde_json::{json, Map, Value};

pub const SCHEMA_PROP_TYPES: &[&str] = &[
    "string", "integer", "number", "boolean", "array", "object",
];

#[derive(Clone, PartialEq, Debug)]
pub struct SchemaPropertyRow {
    pub name: String,
    pub prop_type: String,
    pub example: String,
    pub required: bool,
}

impl SchemaPropertyRow {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            prop_type: "string".into(),
            example: String::new(),
            required: false,
        }
    }
}

pub fn is_plain_schema_object(value: &Value) -> bool {
    value.is_object()
}

pub fn is_editable_object_schema(schema: &Value) -> bool {
    let Some(obj) = schema.as_object() else {
        return false;
    };
    if obj.contains_key("$ref") {
        return false;
    }
    if obj.get("type").and_then(|v| v.as_str()) == Some("array") {
        return false;
    }
    if let Some(t) = obj.get("type").and_then(|v| v.as_str()) {
        if t != "object" {
            return false;
        }
    }
    true
}

fn example_from_prop(prop: &Value) -> String {
    if let Some(d) = prop.get("default") {
        return match d {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    }
    if let Some(ex) = prop.get("example") {
        return match ex {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    }
    if let Some(arr) = prop.get("examples").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first() {
            return match first {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
        }
    }
    String::new()
}

fn coerce_example(prop_type: &str, raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return match prop_type {
            "integer" | "number" => json!(0),
            "boolean" => json!(false),
            "array" => json!([]),
            "object" => json!({}),
            _ => json!(""),
        };
    }
    match prop_type {
        "integer" => trimmed
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or(json!(0)),
        "number" => trimmed
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(json!(0)),
        "boolean" => {
            json!(matches!(trimmed.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        }
        "array" | "object" => serde_json::from_str(trimmed).unwrap_or_else(|_| {
            if prop_type == "array" {
                json!([trimmed])
            } else {
                json!({ "value": trimmed })
            }
        }),
        _ => Value::String(trimmed.to_string()),
    }
}

pub fn schema_to_property_rows(schema: &Value) -> Vec<SchemaPropertyRow> {
    if !is_editable_object_schema(schema) {
        return Vec::new();
    }
    let props = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let required: std::collections::HashSet<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    props
        .into_iter()
        .map(|(name, prop)| {
            let raw_type = prop
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("string");
            let prop_type = if SCHEMA_PROP_TYPES.contains(&raw_type) {
                raw_type.to_string()
            } else {
                "string".into()
            };
            let required = required.contains(&name);
            SchemaPropertyRow {
                name,
                prop_type,
                example: example_from_prop(&prop),
                required,
            }
        })
        .collect()
}

pub fn property_rows_to_schema(rows: &[SchemaPropertyRow]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for row in rows {
        let name = row.name.trim();
        if name.is_empty() {
            continue;
        }
        let mut prop = Map::new();
        prop.insert("type".into(), Value::String(row.prop_type.clone()));
        if !row.example.trim().is_empty() {
            let ex = coerce_example(&row.prop_type, &row.example);
            prop.insert("default".into(), ex.clone());
            prop.insert("example".into(), ex);
        }
        properties.insert(name.to_string(), Value::Object(prop));
        if row.required {
            required.push(Value::String(name.to_string()));
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

pub fn parse_schema_value(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            let raw = s.trim();
            if raw.is_empty() {
                return json!({});
            }
            serde_json::from_str(raw).unwrap_or_else(|_| value.clone())
        }
        Value::Null => json!({}),
        other => other.clone(),
    }
}

/// Placeholder value for a property schema — used for mapping previews.
pub fn sample_value_from_schema(schema: &Value) -> Value {
    if !is_plain_schema_object(schema) {
        return Value::Null;
    }
    if schema.get("$ref").is_some() {
        return json!({});
    }
    if let Some(d) = schema.get("default") {
        return d.clone();
    }
    if let Some(ex) = schema.get("example") {
        return ex.clone();
    }
    if let Some(arr) = schema.get("examples").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first() {
            return first.clone();
        }
    }

    let type_str = schema.get("type").and_then(|v| v.as_str());

    if type_str == Some("array") {
        if let Some(items) = schema.get("items") {
            let one = sample_value_from_schema(items);
            if one.is_null() {
                return json!([]);
            }
            return json!([one]);
        }
        return json!([]);
    }

    if type_str == Some("object") || schema.get("properties").is_some() {
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let mut out = Map::new();
        for (k, v) in props {
            out.insert(k, sample_value_from_schema(&v));
        }
        return Value::Object(out);
    }

    match type_str {
        Some("integer") | Some("number") => json!(0),
        Some("boolean") => json!(false),
        Some("string") => {
            let format = schema.get("format").and_then(|v| v.as_str()).unwrap_or("");
            match format {
                "uuid" => json!("00000000-0000-0000-0000-000000000000"),
                "date-time" => json!("2026-01-01T00:00:00.000Z"),
                "date" => json!("2026-01-01"),
                _ => json!(""),
            }
        }
        _ => Value::Null,
    }
}

/// n8n-style sample item for an apiEndpoint trigger from request schemas.
/// Always returns a request-shaped item so `$json.method` / `$json.query` are
/// mappable even before Query Params / Body fields are defined.
pub fn sample_from_api_endpoint_parameters(parameters: &Value) -> Option<Value> {
    let query_schema = parameters.get("requestQuerySchema");
    let body_schema = parameters.get("requestBodySchema");
    let query_props = query_schema
        .filter(|s| is_plain_schema_object(s))
        .and_then(|s| s.get("properties"))
        .and_then(|v| v.as_object());
    let body_props = body_schema
        .filter(|s| is_plain_schema_object(s))
        .and_then(|s| s.get("properties"))
        .and_then(|v| v.as_object());
    let has_query = query_props.map(|p| !p.is_empty()).unwrap_or(false)
        || query_schema
            .filter(|s| is_plain_schema_object(s))
            .map(|s| s.get("$ref").is_some())
            .unwrap_or(false);
    let has_body = body_props.map(|p| !p.is_empty()).unwrap_or(false)
        || body_schema
            .filter(|s| is_plain_schema_object(s))
            .map(|s| {
                s.get("$ref").is_some()
                    || s.get("type").and_then(|v| v.as_str()) == Some("array")
            })
            .unwrap_or(false);
    let method = parameters
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");
    let path = parameters
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let query = if has_query {
        let q = sample_value_from_schema(query_schema.unwrap_or(&json!({})));
        if q.is_object() {
            q
        } else {
            json!({})
        }
    } else {
        json!({})
    };
    let body = if has_body {
        sample_value_from_schema(body_schema.unwrap_or(&json!({})))
    } else if matches!(
        method.to_ascii_uppercase().as_str(),
        "POST" | "PUT" | "PATCH"
    ) {
        json!({})
    } else {
        Value::Null
    };
    Some(json!([{
        "method": method,
        "path": path,
        "headers": {},
        "query": query,
        "body": body,
    }]))
}
