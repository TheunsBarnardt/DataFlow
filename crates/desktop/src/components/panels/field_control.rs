use dioxus::prelude::*;
use serde_json::Value;

use super::condition_builder::{AssignmentsEditor, ConditionBuilder};
use super::credentials_picker::{is_credential_field, CredentialsPicker};
use super::expression_field::ExpressionField;
use super::key_value_editor::KeyValueEditor;
use super::switch_rules_editor::SwitchRulesEditor;
use super::variable_aware_field::VariableAwareTextarea;
use crate::components::node_schemas::FieldDef;
use super::json_schema_properties_editor::JsonSchemaPropertiesEditor;
use crate::components::ui::{CodeEditor, Switch};
use crate::format_code::CodeLanguage;

fn value_as_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[component]
pub fn FieldControl(
    field: FieldDef,
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
    #[props(default)] on_param_error: Option<EventHandler<String>>,
    /// When set, scopes Monaco/input ids to a node so editors don't share state.
    #[props(default)]
    id_prefix: Option<String>,
) -> Element {
    let id = match &id_prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}-param-{}", field.key),
        _ => format!("param-{}", field.key),
    };

    if is_credential_field(&field.key) || field.credential_type.is_some() {
        let current = value_as_string(&value);
        return rsx! {
            CredentialsPicker {
                id: Some(id),
                value: current,
                credential_type: field.credential_type.clone(),
                on_change: move |next: String| on_change.call(Value::String(next)),
            }
        };
    }

    if field.kind == "boolean" {
        let checked = value.as_bool().unwrap_or(false);
        return rsx! {
            div { class: "flex items-center gap-2",
                Switch {
                    checked,
                    on_checked_change: move |c| on_change.call(Value::Bool(c)),
                }
                label { class: "text-sm", r#for: "{id}", "{field.label}" }
            }
        };
    }

    if field.kind == "select" {
        let current = value
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                field
                    .default
                    .as_ref()
                    .and_then(|v| v.as_str().map(str::to_string))
            })
            .unwrap_or_default();
        let options = field.options.clone().unwrap_or_default();
        return rsx! {
            select {
                id: "{id}",
                key: "{id}-{current}",
                class: "bf-select flex h-9 w-full rounded-md border border-input bg-background px-2 text-sm text-foreground",
                value: "{current}",
                onchange: move |e| on_change.call(Value::String(e.value())),
                for opt in options {
                    option {
                        value: "{opt.value}",
                        selected: current == opt.value,
                        "{opt.label}"
                    }
                }
            }
        };
    }

    if field.kind == "number" {
        let current = match &value {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        return rsx! {
            input {
                id: "{id}",
                r#type: "number",
                class: "flex h-9 w-full rounded-md border border-input bg-background px-3 text-sm",
                value: "{current}",
                placeholder: field.placeholder.clone().unwrap_or_default(),
                oninput: move |e| {
                    let raw = e.value();
                    if raw.trim().is_empty() {
                        on_change.call(Value::Null);
                    } else if let Ok(n) = raw.parse::<f64>() {
                        on_change.call(serde_json::Number::from_f64(n)
                            .map(Value::Number)
                            .unwrap_or(Value::Null));
                    } else {
                        on_change.call(Value::String(raw));
                    }
                },
            }
        };
    }

    if field.kind == "text" {
        let text = value_as_string(&value);
        return rsx! {
            VariableAwareTextarea {
                id: Some(id),
                value: text,
                placeholder: Some(
                    field
                        .placeholder
                        .clone()
                        .unwrap_or_else(|| "Type $ to insert a variable".into()),
                ),
                on_change: move |next: String| on_change.call(Value::String(next)),
            }
        };
    }

    if field.kind == "code" {
        let text = value_as_string(&value);
        let language = CodeLanguage::from_str(field.language.as_deref().unwrap_or("text"));
        let fill = matches!(language, CodeLanguage::Sql);
        let min_h = if fill { 160 } else { 120 };
        // Keep a signal so CM can re-sync when SQL loads after mount.
        let mut code_value = use_signal(|| text.clone());
        if code_value.peek().as_str() != text.as_str() {
            code_value.set(text);
        }
        return rsx! {
            CodeEditor {
                value: code_value,
                language,
                language_hint: field.language.clone(),
                height_px: min_h,
                fill,
                placeholder: field.placeholder.clone(),
                id: Some(id.clone()),
                expr_bus,
                active_field,
                on_change: move |next: String| {
                    code_value.set(next.clone());
                    on_change.call(Value::String(next));
                },
            }
        };
    }

    if field.kind == "jsonSchema" {
        return rsx! {
            JsonSchemaPropertiesEditor {
                value: value.clone(),
                on_change,
                on_format_error: on_param_error,
            }
        };
    }

    if field.kind == "json" {
        let text = match &value {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
        };
        let mut json_value = use_signal(|| text.clone());
        if json_value.peek().as_str() != text.as_str() {
            json_value.set(text);
        }
        return rsx! {
            CodeEditor {
                value: json_value,
                language: CodeLanguage::Json,
                height_px: 120,
                placeholder: Some(field.placeholder.clone().unwrap_or_else(|| "{ }".into())),
                id: Some(id.clone()),
                expr_bus,
                active_field,
                on_change: move |next: String| {
                    json_value.set(next.clone());
                    let raw = next.trim();
                    if raw.is_empty() {
                        on_change.call(Value::Null);
                        return;
                    }
                    match serde_json::from_str::<Value>(raw) {
                        Ok(parsed) => on_change.call(parsed),
                        Err(_) => on_change.call(Value::String(next)),
                    }
                },
            }
        };
    }

    if field.kind == "keyValue" {
        let obj = if value.is_object() {
            value.clone()
        } else {
            Value::Object(Default::default())
        };
        return rsx! {
            KeyValueEditor {
                value: obj,
                on_change,
                expr_bus,
                active_field,
                id_prefix: Some(id.clone()),
            }
        };
    }

    if field.kind == "stringList" {
        let text = value
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        return rsx! {
            textarea {
                id: "{id}",
                class: "min-h-[72px] w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs",
                value: "{text}",
                placeholder: field.placeholder.clone().unwrap_or_else(|| "One per line".into()),
                spellcheck: false,
                oninput: move |e| {
                    let list: Vec<Value> = e
                        .value()
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    on_change.call(Value::Array(list));
                },
            }
        };
    }

    if field.kind == "conditions" {
        return rsx! {
            ConditionBuilder {
                value: value.clone(),
                on_change,
                expr_bus,
                active_field,
                id_prefix: Some(id.clone()),
            }
        };
    }

    if field.kind == "switchRules" {
        return rsx! {
            SwitchRulesEditor {
                value: value.clone(),
                on_change,
                expr_bus,
                active_field,
                id_prefix: Some(id.clone()),
            }
        };
    }

    if field.kind == "assignments" {
        return rsx! {
            AssignmentsEditor {
                value: value.clone(),
                on_change,
                expr_bus,
                active_field,
            }
        };
    }

    // Default / string → ExpressionField
    let text = value_as_string(&value);
    rsx! {
        ExpressionField {
            id: Some(id),
            value: text,
            placeholder: field.placeholder.clone(),
            expr_bus,
            active_field,
            on_change: move |next: String| on_change.call(Value::String(next)),
        }
    }
}
