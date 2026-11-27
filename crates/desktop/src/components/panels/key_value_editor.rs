use dioxus::prelude::*;
use serde_json::{json, Map, Value};

use super::expression_field::ExpressionField;
use crate::components::ui::{Button, ButtonSize, ButtonVariant, Input};

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
        div { class: "flex flex-col gap-2",
            for (i, (k, v)) in entries.iter().cloned().enumerate() {
                {
                    let entries_all = entries.clone();
                    let entries_upd = entries.clone();
                    let entries_rm = entries.clone();
                    let field_id = format!("{id_prefix}-kv-{i}-value");
                    rsx! {
                        div {
                            key: "{i}",
                            class: "flex min-w-0 flex-col gap-1.5 rounded-md border border-border/60 bg-muted/10 p-2",
                            div { class: "flex items-center gap-1.5",
                                Input {
                                    class: Some("h-8 min-w-0 flex-1 text-xs".into()),
                                    placeholder: Some("Key".into()),
                                    value: k.clone(),
                                    oninput: move |e: FormEvent| {
                                        let mut next = entries_upd.clone();
                                        if let Some(row) = next.get_mut(i) {
                                            row.0 = e.value();
                                        }
                                        set_entries(next);
                                    },
                                }
                                Button {
                                    size: ButtonSize::Sm,
                                    variant: ButtonVariant::Ghost,
                                    class: Some("h-8 shrink-0 px-2 text-xs".into()),
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
                            div { class: "min-w-0",
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
                        }
                    }
                }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Outline,
                class: Some("h-8 text-xs".into()),
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