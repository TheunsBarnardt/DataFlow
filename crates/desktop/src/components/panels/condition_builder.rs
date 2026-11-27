use dioxus::prelude::*;
use serde_json::{json, Value};

use super::expression_field::ExpressionField;
use crate::components::ui::Switch;

const COMPARE_OPS: &[(&str, &str)] = &[
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
];

fn normalize(value: &Value) -> Value {
    if let Some(obj) = value.as_object() {
        let combinator = obj
            .get("combinator")
            .and_then(|v| v.as_str())
            .unwrap_or("AND");
        let conditions = obj
            .get("conditions")
            .cloned()
            .unwrap_or_else(|| json!([]));
        let ignore_case = obj
            .get("ignoreCase")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return json!({
            "combinator": combinator,
            "conditions": conditions,
            "ignoreCase": ignore_case,
        });
    }
    if value.is_array() {
        return json!({
            "combinator": "AND",
            "conditions": value,
            "ignoreCase": false,
        });
    }
    json!({
        "combinator": "AND",
        "conditions": [],
        "ignoreCase": false,
    })
}

#[component]
pub fn ConditionBuilder(
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
    #[props(default)] id_prefix: Option<String>,
) -> Element {
    let id_prefix = id_prefix.unwrap_or_else(|| "conditions".into());
    let group = normalize(&value);
    let combinator = group
        .get("combinator")
        .and_then(|v| v.as_str())
        .unwrap_or("AND")
        .to_string();
    let ignore_case = group
        .get("ignoreCase")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let rows = group
        .get("conditions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let emit = move |combinator: String, ignore_case: bool, rows: Vec<Value>| {
        on_change.call(json!({
            "combinator": combinator,
            "ignoreCase": ignore_case,
            "conditions": rows,
        }));
    };

    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center gap-2",
                select {
                    class: "flex h-8 min-w-[4.5rem] rounded-md border border-input bg-background px-2 text-xs",
                    value: "{combinator}",
                    onchange: {
                        let rows = rows.clone();
                        move |e| emit(e.value(), ignore_case, rows.clone())
                    },
                    option { value: "AND", "AND" }
                    option { value: "OR", "OR" }
                }
                div { class: "flex items-center gap-1.5",
                    Switch {
                        checked: ignore_case,
                        size: Some("sm".into()),
                        on_checked_change: {
                            let combinator = combinator.clone();
                            let rows = rows.clone();
                            move |checked| emit(combinator.clone(), checked, rows.clone())
                        },
                    }
                    span { class: "text-[11px] text-muted-foreground", "Ignore case" }
                }
            }
            for (i, row) in rows.iter().cloned().enumerate() {
                {
                    let field = row.get("field").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let operator = row
                        .get("operator")
                        .and_then(|v| v.as_str())
                        .unwrap_or("equals")
                        .to_string();
                    let val = match row.get("value") {
                        Some(Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => String::new(),
                    };
                    let combinator = combinator.clone();
                    let rows_all = rows.clone();
                    rsx! {
                        div {
                            key: "{i}",
                            class: "flex min-w-0 flex-col gap-1.5 rounded-md border border-border/60 bg-muted/10 p-2",
                            div { class: "flex flex-wrap items-center gap-1.5",
                                input {
                                    class: "flex h-8 min-w-[7rem] flex-1 rounded-md border border-input bg-background px-2 font-mono text-xs",
                                    placeholder: "Field / expression",
                                    value: "{field}",
                                    oninput: {
                                        let combinator = combinator.clone();
                                        let mut rows = rows_all.clone();
                                        move |e| {
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("field".into(), Value::String(e.value()));
                                                }
                                            }
                                            emit(combinator.clone(), ignore_case, rows.clone());
                                        }
                                    },
                                }
                                select {
                                    class: "flex h-8 shrink-0 rounded-md border border-input bg-background px-2 text-xs",
                                    value: "{operator}",
                                    onchange: {
                                        let combinator = combinator.clone();
                                        let mut rows = rows_all.clone();
                                        move |e| {
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("operator".into(), Value::String(e.value()));
                                                }
                                            }
                                            emit(combinator.clone(), ignore_case, rows.clone());
                                        }
                                    },
                                    for (ov, ol) in COMPARE_OPS {
                                        option { value: "{ov}", "{ol}" }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "inline-flex h-8 shrink-0 items-center rounded-md px-2 text-xs text-muted-foreground hover:bg-muted",
                                    onclick: {
                                        let combinator = combinator.clone();
                                        let rows = rows_all.clone();
                                        move |_| {
                                            let next: Vec<_> = rows
                                                .iter()
                                                .enumerate()
                                                .filter(|(j, _)| *j != i)
                                                .map(|(_, r)| r.clone())
                                                .collect();
                                            emit(combinator.clone(), ignore_case, next);
                                        }
                                    },
                                    "Remove"
                                }
                            }
                            div { class: "min-w-0",
                                ExpressionField {
                                    id: Some(format!("{id_prefix}-cond-{i}-value")),
                                    value: val,
                                    placeholder: Some("Value".into()),
                                    expr_bus,
                                    active_field,
                                    on_change: {
                                        let combinator = combinator.clone();
                                        let mut rows = rows_all.clone();
                                        move |next: String| {
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("value".into(), Value::String(next));
                                                }
                                            }
                                            emit(combinator.clone(), ignore_case, rows.clone());
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "inline-flex h-8 items-center justify-center rounded-md border px-3 text-xs hover:bg-muted",
                onclick: {
                    let combinator = combinator.clone();
                    let mut rows = rows.clone();
                    move |_| {
                        rows.push(json!({
                            "field": "",
                            "operator": "equals",
                            "value": "",
                        }));
                        emit(combinator.clone(), ignore_case, rows.clone());
                    }
                },
                "Add condition"
            }
        }
    }
}

const ASSIGNMENT_TYPES: &[(&str, &str)] = &[
    ("string", "String"),
    ("number", "Number"),
    ("boolean", "Boolean"),
    ("array", "Array"),
    ("object", "Object"),
];

fn normalize_assignments(value: &Value) -> Vec<Value> {
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }
    if let Some(obj) = value.as_object() {
        return obj
            .iter()
            .map(|(name, v)| {
                json!({
                    "name": name,
                    "value": v,
                    "type": "string",
                })
            })
            .collect();
    }
    Vec::new()
}

fn assignment_value_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

#[component]
pub fn AssignmentsEditor(
    value: Value,
    on_change: EventHandler<Value>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
) -> Element {
    let rows = normalize_assignments(&value);

    let emit = move |rows: Vec<Value>| {
        on_change.call(Value::Array(rows));
    };

    rsx! {
        div { class: "flex flex-col gap-1.5",
            for (i, row) in rows.iter().cloned().enumerate() {
                {
                    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let ty = row
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("string")
                        .to_string();
                    let val = assignment_value_text(row.get("value"));
                    let rows_all = rows.clone();
                    let field_id = format!("assignment-{i}-value");
                    rsx! {
                        div {
                            key: "{i}",
                            class: "flex min-w-0 flex-col gap-1.5 rounded-md border border-border/60 bg-muted/10 p-2",
                            div { class: "flex items-center gap-1.5",
                                input {
                                    class: "flex h-8 min-w-0 flex-1 rounded-md border border-input bg-background px-2 text-xs",
                                    placeholder: "Name",
                                    value: "{name}",
                                    oninput: {
                                        let rows_all = rows_all.clone();
                                        move |e| {
                                            let mut rows = rows_all.clone();
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("name".into(), Value::String(e.value()));
                                                }
                                            }
                                            emit(rows);
                                        }
                                    },
                                }
                                select {
                                    class: "flex h-8 w-auto min-w-[5.5rem] shrink-0 rounded-md border border-input bg-background px-2 text-xs",
                                    value: "{ty}",
                                    onchange: {
                                        let rows_all = rows_all.clone();
                                        move |e| {
                                            let mut rows = rows_all.clone();
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("type".into(), Value::String(e.value()));
                                                }
                                            }
                                            emit(rows);
                                        }
                                    },
                                    for (tv, tl) in ASSIGNMENT_TYPES {
                                        option { value: "{tv}", "{tl}" }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "inline-flex h-8 shrink-0 items-center rounded-md px-2 text-xs text-muted-foreground hover:bg-muted",
                                    aria_label: "Remove",
                                    onclick: {
                                        let rows = rows_all.clone();
                                        move |_| {
                                            let next: Vec<_> = rows
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
                            div { class: "min-w-0",
                                ExpressionField {
                                    id: Some(field_id),
                                    value: val,
                                    placeholder: Some("Value".into()),
                                    expr_bus,
                                    active_field,
                                    on_change: {
                                        let rows_all = rows_all.clone();
                                        move |next: String| {
                                            let mut rows = rows_all.clone();
                                            if let Some(r) = rows.get_mut(i) {
                                                if let Some(obj) = r.as_object_mut() {
                                                    obj.insert("value".into(), Value::String(next));
                                                }
                                            }
                                            emit(rows);
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "inline-flex h-8 items-center justify-center rounded-md border px-3 text-xs hover:bg-muted",
                onclick: {
                    let rows_all = rows.clone();
                    move |_| {
                        let mut next = rows_all.clone();
                        next.push(json!({
                            "name": "",
                            "value": "",
                            "type": "string",
                        }));
                        emit(next);
                    }
                },
                "Add field"
            }
        }
    }
}
