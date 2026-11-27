use dioxus::prelude::*;
use serde_json::Value;

use super::json_schema_sample::{
    is_editable_object_schema, is_plain_schema_object, parse_schema_value, property_rows_to_schema,
    schema_to_property_rows, SchemaPropertyRow, SCHEMA_PROP_TYPES,
};
use crate::components::ui::CodeEditor;
use crate::format_code::{format_code, CodeLanguage};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Fields,
    Json,
}

fn rows_from_parsed(parsed: &Value) -> Vec<SchemaPropertyRow> {
    let mut rows = schema_to_property_rows(parsed);
    if rows.is_empty() {
        rows.push(SchemaPropertyRow::empty());
    } else if rows.last().map(|r| !r.name.trim().is_empty()).unwrap_or(false) {
        rows.push(SchemaPropertyRow::empty());
    }
    rows
}

#[component]
pub fn JsonSchemaPropertiesEditor(
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] on_format_error: Option<EventHandler<String>>,
) -> Element {
    let parsed = parse_schema_value(&value);
    let can_edit_fields = is_editable_object_schema(&parsed);
    let row_list = rows_from_parsed(&parsed);

    let mut mode = use_signal(|| {
        if can_edit_fields {
            Mode::Fields
        } else {
            Mode::Json
        }
    });

    let json_text_str = match &value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    let mut json_text = use_signal(|| json_text_str.clone());
    if json_text.peek().as_str() != json_text_str.as_str() {
        json_text.set(json_text_str.clone());
    }

    let emit_rows = move |next_rows: Vec<SchemaPropertyRow>| {
        let cleaned: Vec<_> = next_rows
            .iter()
            .enumerate()
            .filter(|(i, r)| {
                !r.name.trim().is_empty()
                    || !r.example.trim().is_empty()
                    || r.required
                    || *i + 1 < next_rows.len()
            })
            .map(|(_, r)| r.clone())
            .collect();
        let named: Vec<_> = cleaned
            .into_iter()
            .filter(|r| !r.name.trim().is_empty())
            .collect();
        on_change.call(property_rows_to_schema(&named));
    };

    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex items-center justify-between gap-2",
                div { class: "flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5",
                    button {
                        r#type: "button",
                        class: if mode() == Mode::Fields {
                            "rounded px-2 py-0.5 text-[10px] font-medium bg-card text-foreground shadow-sm"
                        } else {
                            "rounded px-2 py-0.5 text-[10px] font-medium text-muted-foreground"
                        },
                        onclick: move |_| {
                            if !can_edit_fields {
                                on_change.call(property_rows_to_schema(&[]));
                            }
                            mode.set(Mode::Fields);
                        },
                        "Params"
                    }
                    button {
                        r#type: "button",
                        class: if mode() == Mode::Json {
                            "rounded px-2 py-0.5 text-[10px] font-medium bg-card text-foreground shadow-sm"
                        } else {
                            "rounded px-2 py-0.5 text-[10px] font-medium text-muted-foreground"
                        },
                        onclick: move |_| mode.set(Mode::Json),
                        "JSON"
                    }
                }
                if mode() == Mode::Json {
                    button {
                        r#type: "button",
                        class: "rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-muted",
                        title: "Format",
                        onclick: {
                            let src = json_text_str.clone();
                            move |_| {
                                let result = format_code(&src, CodeLanguage::Json);
                                if !result.ok {
                                    if let Some(msg) = result.error {
                                        if let Some(cb) = on_format_error.as_ref() {
                                            cb.call(msg);
                                        }
                                    }
                                    return;
                                }
                                if let Ok(parsed) = serde_json::from_str::<Value>(&result.text) {
                                    on_change.call(parsed);
                                } else {
                                    on_change.call(Value::String(result.text));
                                }
                            }
                        },
                        "Format"
                    }
                }
            }

            if mode() == Mode::Fields {
                if !can_edit_fields && is_plain_schema_object(&parsed) && parsed.get("$ref").is_some() {
                    p { class: "text-[10px] text-muted-foreground",
                        "Replaced $ref schema with editable properties. Switch back to JSON to restore a $ref if needed."
                    }
                }
                div { class: "overflow-hidden rounded-md border border-border",
                    div { class: "grid grid-cols-[minmax(0,1.2fr)_72px_minmax(0,1.4fr)_36px_28px] gap-1 border-b bg-muted/40 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground",
                        span { "Key" }
                        span { "Type" }
                        span { "Value" }
                        span { class: "text-center", title: "Required", "Req" }
                        span {}
                    }
                    for (i, row) in row_list.iter().cloned().enumerate() {
                        {
                            let rows_all = row_list.clone();
                            let is_placeholder = i + 1 == rows_all.len()
                                && row.name.trim().is_empty()
                                && row.example.trim().is_empty();
                            let row_class = if is_placeholder {
                                "grid grid-cols-[minmax(0,1.2fr)_72px_minmax(0,1.4fr)_36px_28px] items-center gap-1 border-b border-border/60 px-1.5 py-1 last:border-b-0 bg-muted/10"
                            } else {
                                "grid grid-cols-[minmax(0,1.2fr)_72px_minmax(0,1.4fr)_36px_28px] items-center gap-1 border-b border-border/60 px-1.5 py-1 last:border-b-0"
                            };
                            let remove_class = if is_placeholder {
                                "text-[10px] text-muted-foreground invisible"
                            } else {
                                "text-[10px] text-muted-foreground hover:text-destructive"
                            };
                            rsx! {
                                div {
                                    key: "{i}",
                                    class: "{row_class}",
                                    input {
                                        class: "h-8 min-w-0 border-0 bg-transparent px-1.5 text-xs shadow-none focus:outline-none focus:ring-0",
                                        placeholder: "Key",
                                        value: "{row.name}",
                                        oninput: {
                                            let mut next = rows_all.clone();
                                            move |e| {
                                                if let Some(r) = next.get_mut(i) {
                                                    r.name = e.value();
                                                }
                                                if i + 1 == next.len() && next.last().map(|r| !r.name.trim().is_empty()).unwrap_or(false) {
                                                    next.push(SchemaPropertyRow::empty());
                                                }
                                                emit_rows(next.clone());
                                            }
                                        },
                                    }
                                    select {
                                        class: "bf-select h-8 border-0 bg-transparent px-0.5 text-[11px] text-foreground shadow-none focus:outline-none",
                                        value: "{row.prop_type}",
                                        onchange: {
                                            let mut next = rows_all.clone();
                                            move |e| {
                                                if let Some(r) = next.get_mut(i) {
                                                    r.prop_type = e.value();
                                                }
                                                emit_rows(next.clone());
                                            }
                                        },
                                        for t in SCHEMA_PROP_TYPES {
                                            option { value: "{t}", "{t}" }
                                        }
                                    }
                                    input {
                                        class: "h-8 min-w-0 border-0 bg-transparent px-1.5 text-xs shadow-none focus:outline-none focus:ring-0",
                                        placeholder: "Value",
                                        value: "{row.example}",
                                        oninput: {
                                            let mut next = rows_all.clone();
                                            move |e| {
                                                if let Some(r) = next.get_mut(i) {
                                                    r.example = e.value();
                                                }
                                                emit_rows(next.clone());
                                            }
                                        },
                                    }
                                    div { class: "flex justify-center",
                                        input {
                                            r#type: "checkbox",
                                            checked: row.required,
                                            title: "Required",
                                            onchange: {
                                                let mut next = rows_all.clone();
                                                move |_| {
                                                    if let Some(r) = next.get_mut(i) {
                                                        r.required = !r.required;
                                                    }
                                                    emit_rows(next.clone());
                                                }
                                            },
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "{remove_class}",
                                        disabled: is_placeholder,
                                        aria_label: "Remove parameter",
                                        onclick: {
                                            let next: Vec<_> = rows_all
                                                .iter()
                                                .enumerate()
                                                .filter(|(j, _)| *j != i)
                                                .map(|(_, r)| r.clone())
                                                .collect();
                                            move |_| {
                                                let mut next = next.clone();
                                                if next.is_empty() {
                                                    next.push(SchemaPropertyRow::empty());
                                                }
                                                emit_rows(next);
                                            }
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    class: "inline-flex h-7 items-center self-start rounded-md border px-2 text-[10px] hover:bg-muted",
                    onclick: move |_| {
                        let mut next: Vec<_> = row_list
                            .iter()
                            .enumerate()
                            .filter(|(i, r)| {
                                *i + 1 < row_list.len()
                                    || !r.name.trim().is_empty()
                                    || !r.example.trim().is_empty()
                            })
                            .map(|(_, r)| r.clone())
                            .collect();
                        next.push(SchemaPropertyRow::empty());
                        next.push(SchemaPropertyRow::empty());
                        emit_rows(next);
                    },
                    "Add parameter"
                }
                p { class: "text-[10px] text-muted-foreground",
                    "Like Postman: set Key + Value for sample request data. Downstream nodes map them as "
                    code { class: "text-[10px]", "$json.query.*" }
                    " / "
                    code { class: "text-[10px]", "$json.body.*" }
                    "."
                }
            } else {
                CodeEditor {
                    value: json_text,
                    language: CodeLanguage::Json,
                    height_px: 120,
                    show_format: false,
                    placeholder: Some("{ \"type\": \"object\", \"properties\": { \"username\": { \"type\": \"string\", \"example\": \"jdoe\" } } }".into()),
                    on_change: move |next: String| {
                        json_text.set(next.clone());
                        let raw = next.trim();
                        if raw.is_empty() {
                            on_change.call(serde_json::json!({}));
                            return;
                        }
                        match serde_json::from_str::<Value>(raw) {
                            Ok(parsed) => on_change.call(parsed),
                            Err(_) => on_change.call(Value::String(next)),
                        }
                    },
                }
            }
        }
    }
}