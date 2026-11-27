//! NDV-style input/output data browser with drag-into-expression support.

use dioxus::prelude::*;
use serde_json::Value;

use super::binary_viewer::{has_binary_data, BinaryViewer};
use crate::components::ui::{
    CodeEditor, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableWrapper, Tabs,
    TabsList, TabsTrigger,
};
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
    let mut io_tab = use_signal(|| "output".to_string());
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
                Tabs {
                    class: Some("gap-0 w-full".into()),
                    value: Some(io_tab()),
                    on_value_change: move |next: String| io_tab.set(next),
                    TabsList { class: Some("h-7 w-full".into()),
                        TabsTrigger {
                            value: "input".to_string(),
                            class: Some("flex-1 px-2 text-[10px]".into()),
                            "Input"
                        }
                        TabsTrigger {
                            value: "output".to_string(),
                            class: Some("flex-1 px-2 text-[10px]".into()),
                            "Output"
                        }
                    }
                }
            }
            {
                let view_tab = match view() {
                    View::Schema => "schema",
                    View::Table => "table",
                    View::Json => "json",
                }
                .to_string();
                rsx! {
                    Tabs {
                        class: Some("gap-0 w-full".into()),
                        value: Some(view_tab),
                        on_value_change: move |next: String| {
                            view.set(match next.as_str() {
                                "table" => View::Table,
                                "json" => View::Json,
                                _ => View::Schema,
                            });
                        },
                        TabsList { class: Some("h-7 w-full".into()),
                            TabsTrigger {
                                value: "schema".to_string(),
                                class: Some("flex-1 px-2 text-[10px] capitalize".into()),
                                "schema"
                            }
                            TabsTrigger {
                                value: "table".to_string(),
                                class: Some("flex-1 px-2 text-[10px] capitalize".into()),
                                "table"
                            }
                            TabsTrigger {
                                value: "json".to_string(),
                                class: Some("flex-1 px-2 text-[10px] capitalize".into()),
                                "json"
                            }
                        }
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
                TableWrapper { class: Some("min-h-0 flex-1 max-h-none rounded-md".into()),
                    if table_rows.is_empty() {
                        p { class: "p-2 text-[11px] text-muted-foreground", "{empty_hint}" }
                    } else {
                        Table { class: Some("text-[10px] max-w-none".into()),
                            TableHeader { class: Some("bg-muted/80".into()),
                                TableRow {
                                    TableHead { class: Some("h-8 px-2 py-1".into()), "#" }
                                    for c in columns.iter() {
                                        {
                                            let expr = format!("={{{{ {root}.{c} }}}}");
                                            let expr_drag = expr.clone();
                                            let expr_click = expr.clone();
                                            rsx! {
                                                TableHead {
                                                    key: "{c}",
                                                    class: Some("h-8 px-2 py-1".into()),
                                                    span {
                                                        class: "cursor-grab select-none",
                                                        title: "Drag into a parameter",
                                                        draggable: "true",
                                                        "data-bf-expr": "{expr_drag}",
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
                            TableBody {
                                for (i, row) in table_rows.iter().take(50).enumerate() {
                                    TableRow {
                                        key: "{i}",
                                        class: Some("odd:bg-muted/20".into()),
                                        TableCell { class: Some("p-0 px-2 py-1 text-muted-foreground".into()), "{i}" }
                                        for c in columns.iter() {
                                            {
                                                let cell = cell_display(row.get(c));
                                                rsx! {
                                                    TableCell {
                                                        key: "{c}",
                                                        class: Some("max-w-[10rem] truncate p-0 px-2 py-1".into()),
                                                        title: Some(cell.clone()),
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
                                // Use a div — <button draggable> is unreliable in WebView2.
                                div {
                                    role: "button",
                                    tabindex: "0",
                                    class: "flex w-full cursor-grab items-center justify-between gap-2 rounded px-1.5 py-1 text-left hover:bg-muted active:cursor-grabbing",
                                    title: "Drag into a parameter or click to insert",
                                    draggable: "true",
                                    "data-bf-expr": "{f.expr}",
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
                                    span { class: "truncate font-mono text-[11px] pointer-events-none", "{f.path}" }
                                    span { class: "shrink-0 text-[10px] text-muted-foreground pointer-events-none",
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
