use dioxus::prelude::*;
use serde_json::{json, Map, Value};

use super::expression_field::ExpressionField;

#[component]
pub fn KeyValueEditor(
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
    #[props(default)] id_prefix: Option<String>,
) -> Element {
    let id_prefix = id_prefix.unwrap_or_else(|| "key-value".into());

    let entries: Vec<(String, String)> = value
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let set_entries = move |next: Vec<(String, String)>| {
        let mut map = Map::new();
        for (k, v) in next {
            if !k.trim().is_empty() {
                map.insert(k, Value::String(v));
            }
        }
        on_change.call(Value::Object(map));
    };

    rsx! {
        div { class: "flex flex-col gap-1.5",
            for (i, (k, v)) in entries.iter().cloned().enumerate() {
                {
                    let entries_all = entries.clone();
                    let entries_upd = entries.clone();
                    let entries_rm = entries.clone();
                    let field_id = format!("{id_prefix}-kv-{i}-value");
                    rsx! {
                        div {
                            key: "{i}",
                            class: "flex items-center gap-1",
                            input {
                                class: "flex h-8 w-full rounded-md border border-input bg-background px-2 text-xs",
                                placeholder: "Key",
                                value: "{k}",
                                oninput: move |e| {
                                    let mut next = entries_upd.clone();
                                    if let Some(row) = next.get_mut(i) {
                                        row.0 = e.value();
                                    }
                                    set_entries(next);
                                },
                            }
                            div { class: "min-w-0 flex-1",
                                ExpressionField {
                                    id: Some(field_id),
                                    value: v,
                                    placeholder: Some("Value ($ for vars)".into()),
                                    expr_bus,
                                    active_field,
                                    on_change: {
                                        let entries_all = entries_all.clone();
                                        move |next: String| {
                                            let mut rows = entries_all.clone();
                                            if let Some(row) = rows.get_mut(i) {
                                                row.1 = next;
                                            }
                                            set_entries(rows);
                                        }
                                    },
                                }
                            }
                            button {
                                r#type: "button",
                                class: "inline-flex h-8 shrink-0 items-center rounded-md px-2 text-xs text-muted-foreground hover:bg-muted",
                                onclick: move |_| {
                                    let next: Vec<_> = entries_rm
                                        .iter()
                                        .enumerate()
                                        .filter(|(j, _)| *j != i)
                                        .map(|(_, r)| r.clone())
                                        .collect();
                                    set_entries(next);
                                },
                                "Remove"
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "inline-flex h-8 items-center justify-center rounded-md border px-3 text-xs hover:bg-muted",
                onclick: move |_| {
                    let mut next = entries.clone();
                    next.push((String::new(), String::new()));
                    set_entries(next);
                },
                "Add pair"
            }
        }
    }
}

/// Helper for empty object default.
#[allow(dead_code)]
pub fn empty_object() -> Value {
    json!({})
}