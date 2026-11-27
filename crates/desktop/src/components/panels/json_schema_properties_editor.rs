use dioxus::prelude::*;
use serde_json::Value;

use super::json_schema_sample::{
    is_editable_object_schema, is_plain_schema_object, parse_schema_value, property_rows_to_schema,
    schema_to_property_rows, SchemaPropertyRow, SCHEMA_PROP_TYPES,
};
use crate::components::ui::{
    Button, ButtonSize, ButtonVariant, Checkbox, CodeEditor, Input, Select, SelectOption, Table,
    TableBody, TableCell, TableHead, TableHeader, TableRow, TableWrapper, Tabs, TabsList,
    TabsTrigger,
};
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

    let tab_value = if mode() == Mode::Fields {
        "params".to_string()
    } else {
        "json".to_string()
    };

    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex items-center justify-between gap-2",
                Tabs {
                    class: Some("gap-0".into()),
                    value: Some(tab_value),
                    on_value_change: move |next: String| {
                        if next == "params" {
                            if !can_edit_fields {
                                on_change.call(property_rows_to_schema(&[]));
                            }
                            mode.set(Mode::Fields);
                        } else {
                            mode.set(Mode::Json);
                        }
                    },
                    TabsList { class: Some("h-7".into()),
                        TabsTrigger {
                            value: "params".to_string(),
                            class: Some("px-2 text-[10px]".into()),
                            "Params"
                        }
                        TabsTrigger {
                            value: "json".to_string(),
                            class: Some("px-2 text-[10px]".into()),
                            "JSON"
                        }
                    }
                }
                if mode() == Mode::Json {
                    Button {
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Ghost,
                        class: Some("h-7 px-1.5 text-[10px]".into()),
                        title: Some("Format".into()),
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
                TableWrapper { class: Some("overflow-hidden max-h-none border-border".into()),
                    Table { class: Some("text-xs max-w-none table-fixed".into()),
                        TableHeader { class: Some("bg-muted/40 text-[10px] uppercase tracking-wide".into()),
                            TableRow {
                                TableHead { class: Some("h-8 w-[28%] px-1.5 py-1.5".into()), "Key" }
                                TableHead { class: Some("h-8 w-[88px] px-1.5 py-1.5".into()), "Type" }
                                TableHead { class: Some("h-8 w-[36%] px-1.5 py-1.5".into()), "Value" }
                                TableHead {
                                    class: Some("h-8 w-9 px-1 py-1.5 text-center".into()),
                                    title: Some("Required".into()),
                                    "Req"
                                }
                                TableHead { class: Some("h-8 w-7 px-1 py-1.5".into()), "" }
                            }
                        }
                        TableBody {
                            for (i, row) in row_list.iter().cloned().enumerate() {
                                {
                                    let rows_all = row_list.clone();
                                    let is_placeholder = i + 1 == rows_all.len()
                                        && row.name.trim().is_empty()
                                        && row.example.trim().is_empty();
                                    let row_class = if is_placeholder {
                                        "bg-muted/10"
                                    } else {
                                        ""
                                    };
                                    let remove_class = if is_placeholder {
                                        "text-[10px] text-muted-foreground invisible"
                                    } else {
                                        "text-[10px] text-muted-foreground hover:text-destructive"
                                    };
                                    rsx! {
                                        TableRow {
                                            key: "{i}",
                                            class: Some(row_class.into()),
                                            TableCell { class: Some("p-0 px-1.5 py-1".into()),
                                                Input {
                                                    class: Some("h-8 border-0 bg-transparent px-1.5 shadow-none focus-visible:ring-0".into()),
                                                    placeholder: Some("Key".into()),
                                                    value: row.name.clone(),
                                                    oninput: {
                                                        let mut next = rows_all.clone();
                                                        move |e: FormEvent| {
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
                                            }
                                            TableCell { class: Some("p-0 px-0.5 py-1".into()),
                                                Select {
                                                    class: Some("bf-select h-8 border-0 bg-transparent px-0.5 text-[11px] shadow-none focus-visible:ring-0".into()),
                                                    value: row.prop_type.clone(),
                                                    onchange: {
                                                        let mut next = rows_all.clone();
                                                        move |e: FormEvent| {
                                                            if let Some(r) = next.get_mut(i) {
                                                                r.prop_type = e.value();
                                                            }
                                                            emit_rows(next.clone());
                                                        }
                                                    },
                                                    for t in SCHEMA_PROP_TYPES {
                                                        SelectOption { value: (*t).to_string(), "{t}" }
                                                    }
                                                }
                                            }
                                            TableCell { class: Some("p-0 px-1.5 py-1".into()),
                                                Input {
                                                    class: Some("h-8 border-0 bg-transparent px-1.5 shadow-none focus-visible:ring-0".into()),
                                                    placeholder: Some("Value".into()),
                                                    value: row.example.clone(),
                                                    oninput: {
                                                        let mut next = rows_all.clone();
                                                        move |e: FormEvent| {
                                                            if let Some(r) = next.get_mut(i) {
                                                                r.example = e.value();
                                                            }
                                                            emit_rows(next.clone());
                                                        }
                                                    },
                                                }
                                            }
                                            TableCell { class: Some("p-0 py-1".into()),
                                                div { class: "flex justify-center",
                                                    Checkbox {
                                                        checked: row.required,
                                                        title: Some("Required".to_string()),
                                                        aria_label: "Required".to_string(),
                                                        on_checked_change: {
                                                            let mut next = rows_all.clone();
                                                            move |checked: bool| {
                                                                if let Some(r) = next.get_mut(i) {
                                                                    r.required = checked;
                                                                }
                                                                emit_rows(next.clone());
                                                            }
                                                        },
                                                    }
                                                }
                                            }
                                            TableCell { class: Some("p-0 py-1 text-center".into()),
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    variant: ButtonVariant::Ghost,
                                                    class: Some(format!("h-7 w-7 px-0 {remove_class}")),
                                                    disabled: is_placeholder,
                                                    title: Some("Remove parameter".into()),
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
                        }
                    }
                }
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Outline,
                    class: Some("h-7 self-start px-2 text-[10px]".into()),
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