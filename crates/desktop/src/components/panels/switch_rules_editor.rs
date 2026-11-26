use dioxus::prelude::*;
use serde_json::{json, Value};

use super::expression_field::ExpressionField;

#[component]
pub fn SwitchRulesEditor(
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
    #[props(default)] id_prefix: Option<String>,
) -> Element {
    let id_prefix = id_prefix.unwrap_or_else(|| "switch-rules".into());
    let rules: Vec<Value> = value.as_array().cloned().unwrap_or_default();

    let emit = move |next: Vec<Value>| {
        on_change.call(Value::Array(next));
    };

    rsx! {
        div { class: "flex flex-col gap-2",
            for (i, rule) in rules.iter().cloned().enumerate() {
                {
                    let output_key = rule
                        .get("outputKey")
                        .or_else(|| rule.get("output"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let expr = rule
                        .get("expression")
                        .or_else(|| rule.get("value"))
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    let rules_all = rules.clone();
                    rsx! {
                        div {
                            key: "{i}",
                            class: "flex flex-col gap-1 rounded-md border p-2",
                            div { class: "flex items-center gap-1",
                                input {
                                    class: "flex h-8 flex-1 rounded-md border border-input bg-background px-2 text-xs",
                                    placeholder: "Output key",
                                    value: "{output_key}",
                                    oninput: {
                                        let mut rules = rules_all.clone();
                                        move |e| {
                                            if let Some(r) = rules.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert(
                                                        "outputKey".into(),
                                                        Value::String(e.value()),
                                                    );
                                                }
                                            }
                                            emit(rules.clone());
                                        }
                                    },
                                }
                                button {
                                    r#type: "button",
                                    class: "inline-flex h-8 items-center rounded-md px-2 text-xs text-muted-foreground hover:bg-muted",
                                    onclick: {
                                        let rules = rules_all.clone();
                                        move |_| {
                                            let next: Vec<_> = rules
                                                .iter()
                                                .enumerate()
                                                .filter(|(j, _)| *j != i)
                                                .map(|(_, r)| r.clone())
                                                .collect();
                                            emit(next);
                                        }
                                    },
                                    "Remove"
                                }
                            }
                            ExpressionField {
                                id: Some(format!("{id_prefix}-rule-{i}")),
                                value: expr,
                                multiline: true,
                                placeholder: Some("Rule expression".into()),
                                expr_bus,
                                active_field,
                                on_change: {
                                    let mut rules = rules_all.clone();
                                    move |next: String| {
                                        if let Some(r) = rules.get_mut(i) {
                                            if let Some(obj) = r.as_object_mut() {
                                                obj.insert("expression".into(), Value::String(next));
                                            }
                                        }
                                        emit(rules.clone());
                                    }
                                },
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "inline-flex h-8 items-center justify-center rounded-md border px-3 text-xs hover:bg-muted",
                onclick: {
                    let mut rules = rules.clone();
                    move |_| {
                        rules.push(json!({
                            "outputKey": format!("out{}", rules.len() + 1),
                            "expression": "",
                        }));
                        emit(rules.clone());
                    }
                },
                "Add rule"
            }
        }
    }
}
