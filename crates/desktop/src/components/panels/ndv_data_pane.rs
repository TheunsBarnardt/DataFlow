//! NDV-style input/output data browser with drag-into-expression support.

use dioxus::prelude::*;
use serde_json::Value;

use super::binary_viewer::{has_binary_data, BinaryViewer};
use crate::components::ui::CodeEditor;
use crate::format_code::CodeLanguage;

pub const DF_DRAG_MIME: &str = "application/x-dataflow-expr";

pub fn normalize_drag_expr(expr: &str) -> String {
    let t = expr.trim();
    if t.starts_with("={{") {
        return t.to_string();
    }
    if t.starts_with("{{") {
        return format!("={t}");
    }
    format!("={{ {t} }}")
}

fn set_drag_transfer(e: &Event<DragData>, expr: &str) {
    let dt = e.data().data_transfer();
    let _ = dt.set_data(DF_DRAG_MIME, expr);
    let _ = dt.set_data("text/plain", expr);
    dt.set_effect_allowed("copy");
}

#[derive(Clone, PartialEq)]
struct PathField {
    path: String,
    expr: String,
    preview: String,
    type_name: String,
}

fn type_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn preview_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn is_envelope(item: &Value) -> bool {
    let Some(obj) = item.as_object() else {
        return false;
    };
    if !obj.contains_key("json") || !obj.get("json").map(|v| v.is_object()).unwrap_or(false) {
        return false;
    }
    obj.contains_key("pairedItem")
        || obj.contains_key("binary")
        || obj.keys().all(|k| matches!(k.as_str(), "json" | "pairedItem" | "binary"))
}

fn item_json(item: &Value) -> Value {
    let mut cur = item.clone();
    for _ in 0..4 {
        if !is_envelope(&cur) {
            break;
        }
        if let Some(j) = cur.get("json").cloned() {
            cur = j;
        } else {
            break;
        }
    }
    if cur.is_object() {
        cur
    } else {
        serde_json::json!({ "value": cur })
    }
}

/// Unwrap engine log / execute-node payloads into a flat item list.
/// Handles Rust `{ outputs: [{ items }] }`, TS `{ outputs: { main }, items }`,
/// and bare arrays.
pub fn normalize_io_data(data: &Value) -> Value {
    if data.is_null() {
        return Value::Null;
    }
    if data.is_array() {
        return data.clone();
    }
    let Some(obj) = data.as_object() else {
        return Value::Array(vec![data.clone()]);
    };

    // Rust NodeResult serialized onto the run log: { outputs: [ { items, … } ] }
    if let Some(Value::Array(ports)) = obj.get("outputs") {
        let mut items = Vec::new();
        for port in ports {
            if let Some(arr) = port.get("items").and_then(|v| v.as_array()) {
                items.extend(arr.iter().cloned());
            } else if port.is_object() || port.is_array() {
                // Rare: port is already an item / list
                match port {
                    Value::Array(a) => items.extend(a.iter().cloned()),
                    other => items.push(other.clone()),
                }
            }
        }
        if !items.is_empty() {
            return Value::Array(items);
        }
    }

    // TS branched map: { outputs: { main: [...], … }, items?: [...] }
    if let Some(Value::Object(outs)) = obj.get("outputs") {
        if let Some(main) = outs.get("main").cloned() {
            return normalize_io_data(&main);
        }
        if let Some((_, first)) = outs.iter().next() {
            return normalize_io_data(first);
        }
    }

    if let Some(items) = obj.get("items") {
        return normalize_io_data(items);
    }

    // Skipped / waiting markers — keep as single row for JSON tab.
    if obj.contains_key("skipped") || obj.contains_key("waiting") {
        return Value::Array(vec![data.clone()]);
    }

    Value::Array(vec![data.clone()])
}

fn as_items(data: &Value) -> Vec<Value> {
    match normalize_io_data(data) {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr,
        other => vec![other],
    }
}

/// Expands `__raw` / `value` / `rows` / `data` / `items` list payloads into table rows.
fn to_table_rows(items: &[Value]) -> Vec<Value> {
    if items.is_empty() {
        return Vec::new();
    }
    if items.len() == 1 {
        if let Some(obj) = items[0].as_object() {
            for key in ["__raw", "value", "rows", "data", "items"] {
                if let Some(Value::Array(arr)) = obj.get(key) {
                    if !arr.is_empty() && arr.iter().all(|el| el.is_object()) {
                        return arr.clone();
                    }
                }
            }
        }
    }
    items
        .iter()
        .map(|row| {
            if let Some(obj) = row.as_object() {
                if let Some(raw) = obj.get("__raw") {
                    if raw.is_object() && obj.len() <= 2 {
                        return raw.clone();
                    }
                }
            }
            row.clone()
        })
        .collect()
}

fn cell_display(v: Option<&Value>) -> String {
    match v {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

fn flatten_paths(
    value: &Value,
    prefix: &str,
    out: &mut Vec<PathField>,
    depth: usize,
) {
    if depth > 5 {
        return;
    }
    match value {
        Value::Null => {
            out.push(PathField {
                path: prefix.into(),
                expr: format!("={{{{ {prefix} }}}}"),
                preview: "null".into(),
                type_name: "null".into(),
            });
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                out.push(PathField {
                    path: prefix.into(),
                    expr: format!("={{{{ {prefix} }}}}"),
                    preview: "[]".into(),
                    type_name: "array".into(),
                });
            } else {
                flatten_paths(&arr[0], &format!("{prefix}[0]"), out, depth + 1);
            }
        }
        Value::Object(obj) => {
            for (k, v) in obj {
                let next = format!("{prefix}.{k}");
                if v.is_object() || v.is_array() {
                    flatten_paths(v, &next, out, depth + 1);
                } else {
                    out.push(PathField {
                        path: next.clone(),
                        expr: format!("={{{{ {next} }}}}"),
                        preview: preview_of(v),
                        type_name: type_of(v).into(),
                    });
                }
            }
        }
        other => {
            out.push(PathField {
                path: prefix.into(),
                expr: format!("={{{{ {prefix} }}}}"),
                preview: preview_of(other),
                type_name: type_of(other).into(),
            });
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Schema,
    Table,
    Json,
}

#[component]
pub fn NdvDataPane(
    #[props(default = "Output data".to_string())] title: String,
    #[props(default)] data: Option<Value>,
    #[props(default)] input_data: Option<Value>,
    #[props(default = "No data yet. Execute the node or run the workflow.".to_string())]
    empty_hint: String,
    #[props(default)] expr_prefix: Option<String>,
    on_insert_expr: Option<EventHandler<String>>,
) -> Element {
    let mut view = use_signal(|| View::Schema);
    let io_tabs = input_data.is_some() && data.is_some();
    let mut io_tab = use_signal(|| "output");
    let root = expr_prefix.unwrap_or_else(|| "$json".into());
    // Always allocate — Json view mounts Monaco; keep hook order stable across tabs.
    let mut json_value = use_signal(String::new);

    let active_data = if io_tabs {
        if io_tab() == "input" {
            input_data.clone()
        } else {
            data.clone()
        }
    } else {
        data.clone().or(input_data.clone())
    };

    let (schema_fields, json_text, table_rows, columns) = {
        if let Some(ref data) = active_data {
            let raw_items = as_items(data);
            let items: Vec<Value> = raw_items.iter().map(item_json).collect();
            let table_rows = to_table_rows(&items);
            let first = table_rows
                .first()
                .or_else(|| items.first())
                .cloned()
                .unwrap_or(Value::Null);
            let mut fields = Vec::new();
            if !first.is_null() {
                flatten_paths(&first, &root, &mut fields, 0);
            }
            let display: Vec<Value> = raw_items
                .iter()
                .enumerate()
                .map(|(i, raw)| {
                    let mut obj = serde_json::Map::new();
                    obj.insert(
                        "json".into(),
                        items.get(i).cloned().unwrap_or(Value::Object(Default::default())),
                    );
                    if let Some(p) = raw.get("pairedItem") {
                        obj.insert("pairedItem".into(), p.clone());
                    }
                    if let Some(b) = raw.get("binary") {
                        obj.insert("binary".into(), b.clone());
                    }
                    Value::Object(obj)
                })
                .collect();
            let json_text =
                serde_json::to_string_pretty(&display).unwrap_or_else(|_| "[]".into());
            let rows = table_rows;
            let mut cols = std::collections::BTreeSet::new();
            for row in &rows {
                if let Some(obj) = row.as_object() {
                    for k in obj.keys() {
                        if k != "pairedItem" && k != "binary" {
                            cols.insert(k.clone());
                        }
                    }
                }
            }
            let columns: Vec<String> = cols.into_iter().take(24).collect();
            (fields, json_text, rows, columns)
        } else {
            (Vec::new(), String::new(), Vec::new(), Vec::new())
        }
    };

    if json_value.peek().as_str() != json_text.as_str() {
        json_value.set(json_text.clone());
    }
    let json_editor_id = format!("ndv-json-{}", io_tab());

    rsx! {
        div { class: "flex min-h-0 flex-1 flex-col gap-2",
            div { class: "text-[11px] font-semibold uppercase tracking-wide text-muted-foreground",
                "{title}"
            }
            if io_tabs {
                div { class: "flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5",
                    button {
                        r#type: "button",
                        class: if io_tab() == "input" {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium bg-card text-foreground shadow-sm"
                        } else {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium text-muted-foreground"
                        },
                        onclick: move |_| io_tab.set("input"),
                        "Input"
                    }
                    button {
                        r#type: "button",
                        class: if io_tab() == "output" {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium bg-card text-foreground shadow-sm"
                        } else {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium text-muted-foreground"
                        },
                        onclick: move |_| io_tab.set("output"),
                        "Output"
                    }
                }
            }
            div { class: "flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5",
                for (v, label) in [(View::Schema, "schema"), (View::Table, "table"), (View::Json, "json")] {
                    button {
                        r#type: "button",
                        class: if view() == v {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium capitalize bg-card text-foreground shadow-sm"
                        } else {
                            "flex-1 rounded px-2 py-1 text-[10px] font-medium capitalize text-muted-foreground"
                        },
                        onclick: move |_| view.set(v),
                        "{label}"
                    }
                }
            }

            if active_data.is_none() {
                p { class: "text-[11px] text-muted-foreground", "{empty_hint}" }
            } else if view() == View::Json {
                div { class: "flex min-h-0 flex-1 flex-col",
                    CodeEditor {
                        value: json_value,
                        language: CodeLanguage::Json,
                        fill: true,
                        read_only: true,
                        show_format: false,
                        id: Some(json_editor_id),
                        on_change: move |_| {},
                    }
                }
            } else if view() == View::Table {
                div { class: "min-h-0 flex-1 overflow-auto rounded-md border",
                    if table_rows.is_empty() {
                        p { class: "p-2 text-[11px] text-muted-foreground", "{empty_hint}" }
                    } else {
                        table { class: "w-full border-collapse text-left text-[10px]",
                            thead { class: "sticky top-0 bg-muted/80",
                                tr {
                                    th { class: "border-b px-2 py-1 font-medium", "#" }
                                    for c in columns.iter() {
                                        {
                                            let expr = format!("={{{{ {root}.{c} }}}}");
                                            let expr_drag = expr.clone();
                                            let expr_click = expr.clone();
                                            rsx! {
                                                th {
                                                    key: "{c}",
                                                    class: "border-b px-2 py-1 font-medium",
                                                    span {
                                                        class: "cursor-grab",
                                                        title: "Drag into a parameter",
                                                        draggable: true,
                                                        ondragstart: move |e| set_drag_transfer(&e, &expr_drag),
                                                        onclick: move |_| {
                                                            if let Some(cb) = on_insert_expr.as_ref() {
                                                                cb.call(expr_click.clone());
                                                            }
                                                        },
                                                        "{c}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            tbody {
                                for (i, row) in table_rows.iter().take(50).enumerate() {
                                    tr {
                                        key: "{i}",
                                        class: "odd:bg-muted/20",
                                        td { class: "border-b px-2 py-1 text-muted-foreground", "{i}" }
                                        for c in columns.iter() {
                                            {
                                                let cell = cell_display(row.get(c));
                                                rsx! {
                                                    td {
                                                        key: "{c}",
                                                        class: "max-w-[10rem] truncate border-b px-2 py-1",
                                                        title: "{cell}",
                                                        "{cell}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                ul { class: "flex min-h-0 flex-1 flex-col gap-0.5 overflow-auto rounded-md border bg-muted/20 p-1.5",
                    if schema_fields.is_empty() {
                        p { class: "px-1 text-[11px] text-muted-foreground", "{empty_hint}" }
                    } else {
                        for f in schema_fields.iter().take(60).cloned() {
                            li {
                                key: "{f.path}",
                                button {
                                    r#type: "button",
                                    class: "flex w-full cursor-grab items-center justify-between gap-2 rounded px-1.5 py-1 text-left hover:bg-muted active:cursor-grabbing",
                                    title: "Drag into a parameter or click to insert",
                                    draggable: true,
                                    ondragstart: {
                                        let expr = f.expr.clone();
                                        move |e| set_drag_transfer(&e, &expr)
                                    },
                                    onclick: {
                                        let expr = f.expr.clone();
                                        move |_| {
                                            if let Some(cb) = on_insert_expr.as_ref() {
                                                cb.call(expr.clone());
                                            }
                                        }
                                    },
                                    span { class: "truncate font-mono text-[11px]", "{f.path}" }
                                    span { class: "shrink-0 text-[10px] text-muted-foreground",
                                        "{f.type_name} · {f.preview.chars().take(24).collect::<String>()}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(ref d) = active_data {
                if has_binary_data(d) {
                    BinaryViewer { data: d.clone() }
                }
            }
        }
    }
}
