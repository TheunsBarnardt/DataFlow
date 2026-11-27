use dioxus::prelude::*;
use dioxus_flow::prelude::Node;
use serde_json::{json, Value};

use super::json_schema_sample::sample_from_api_endpoint_parameters;
use crate::components::flow::node_glyphs::NodeGlyph;
use crate::components::flow::StudioNodeData;
use crate::components::icons::{ArrowLeft, ChevronLeft, ChevronRight, Play, Trash2, X};
use crate::components::node_schemas::{field_visible, get_schema, merge_defaults, sanitize_parameters};
use crate::components::ui::{
    Button, ButtonSize, ButtonVariant, ConfirmDeleteDialog, Input, Label, ResizableHandle,
    ResizableOrientation, ResizablePanel, ResizablePanelGroup, Select, SelectOption, Switch,
    Textarea,
};

use super::field_control::FieldControl;
use super::ndv_data_pane::{normalize_io_data, NdvDataPane};
use super::webhook_listen_panel::{WebhookListenPanel, WebhookSample};

fn default_settings() -> Value {
    json!({
        "alwaysOutputData": false,
        "executeOnce": false,
        "retryOnFail": false,
        "maxTries": 3,
        "waitBetweenTries": 1000,
        "onError": "stopWorkflow",
        "notes": "",
        "displayNoteInFlow": false,
        "disabled": false,
    })
}

fn merge_settings(doc: &Value) -> Value {
    let defaults = default_settings();
    let Some(existing) = doc.get("settings") else {
        return defaults;
    };
    let mut merged = defaults
        .as_object()
        .cloned()
        .unwrap_or_default();
    if let Some(obj) = existing.as_object() {
        for (k, v) in obj {
            merged.insert(k.clone(), v.clone());
        }
    }
    Value::Object(merged)
}

fn as_items(data: &Value) -> Vec<Value> {
    match normalize_io_data(data) {
        Value::Null => Vec::new(),
        Value::Array(arr) => arr,
        other => vec![other],
    }
}

/// True when data is null, empty, or only null items — not useful for NDV mapping.
fn is_nullish_io(data: &Value) -> bool {
    match normalize_io_data(data) {
        Value::Null => true,
        Value::Array(arr) => arr.is_empty() || arr.iter().all(|v| v.is_null()),
        _ => false,
    }
}

fn usable_io(data: &Value) -> Option<Value> {
    let normalized = normalize_io_data(data);
    if is_nullish_io(&normalized) {
        None
    } else {
        Some(normalized)
    }
}

fn log_for_node<'a>(node_id: &str, run_logs: &'a [Value]) -> Option<&'a Value> {
    run_logs
        .iter()
        .rev()
        .find(|l| l.get("nodeId").and_then(|v| v.as_str()) == Some(node_id))
}

fn output_for_node(
    node_id: &str,
    run_logs: &[Value],
    node_samples: &std::collections::HashMap<String, Value>,
    sample_output: &Option<Value>,
) -> Option<Value> {
    if let Some(s) = sample_output {
        if let Some(v) = usable_io(s) {
            return Some(v);
        }
    }
    if let Some(s) = node_samples.get(node_id) {
        if let Some(v) = usable_io(s) {
            return Some(v);
        }
    }
    log_for_node(node_id, run_logs)
        .and_then(|l| l.get("output"))
        .and_then(usable_io)
}

/// React `detailsInputSample` parity: log.input → upstream output/sample → apiEndpoint schema.
fn input_for_node(
    node_id: &str,
    node_type: &str,
    parameters: &Value,
    run_logs: &[Value],
    node_samples: &std::collections::HashMap<String, Value>,
    doc_nodes: &[Value],
    doc_edges: &[Value],
) -> Option<Value> {
    if let Some(input) = log_for_node(node_id, run_logs).and_then(|l| l.get("input")) {
        if let Some(v) = usable_io(input) {
            return Some(v);
        }
    }

    let upstream = doc_edges.iter().find_map(|e| {
        let target = e.get("target").and_then(|v| v.as_str())?;
        if target != node_id {
            return None;
        }
        e.get("source").and_then(|v| v.as_str()).map(String::from)
    });

    if let Some(source_id) = upstream {
        if let Some(out) = log_for_node(&source_id, run_logs).and_then(|l| l.get("output")) {
            if let Some(v) = usable_io(out) {
                return Some(v);
            }
        }
        if let Some(s) = node_samples.get(&source_id) {
            if let Some(v) = usable_io(s) {
                return Some(v);
            }
        }
        if let Some(up) = doc_nodes.iter().find(|n| {
            n.get("id").and_then(|v| v.as_str()) == Some(source_id.as_str())
        }) {
            let up_ty = up.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if up_ty == "apiEndpoint" {
                let params = up
                    .get("parameters")
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                if let Some(sample) = sample_from_api_endpoint_parameters(&params) {
                    return Some(sample);
                }
            }
        }
        return None;
    }

    if node_type == "apiEndpoint" {
        return sample_from_api_endpoint_parameters(parameters);
    }
    None
}

fn emit_parameters_update(
    id: String,
    node_type: &str,
    params: Value,
    on_update: EventHandler<NodeUpdate>,
    on_param_error: Option<EventHandler<String>>,
) {
    let parameters = if let Some(schema) = get_schema(node_type) {
        match sanitize_parameters(schema, &params) {
            Ok(clean) => clean,
            Err(e) => {
                if let Some(handler) = on_param_error {
                    handler.call(e);
                }
                return;
            }
        }
    } else {
        params
    };
    on_update.call(NodeUpdate {
        id,
        name: None,
        parameters: Some(parameters),
        parameter_patch: None,
        settings: None,
        delete: false,
    });
}

fn emit_parameter_patch(
    id: String,
    key: String,
    value: Value,
    on_update: EventHandler<NodeUpdate>,
) {
    on_update.call(NodeUpdate {
        id,
        name: None,
        parameters: None,
        parameter_patch: Some((key, value)),
        settings: None,
        delete: false,
    });
}

fn patch_settings(
    id: String,
    mut settings: Value,
    key: &str,
    value: Value,
    on_update: EventHandler<NodeUpdate>,
) {
    if let Some(obj) = settings.as_object_mut() {
        obj.insert(key.into(), value);
    }
    on_update.call(NodeUpdate {
        id,
        name: None,
        parameters: None,
        parameter_patch: None,
        settings: Some(settings),
        delete: false,
    });
}

#[derive(Clone, PartialEq)]
pub struct NodeUpdate {
    pub id: String,
    pub name: Option<String>,
    /// Full parameters replace (avoid for single-field edits — races wipe sibling keys).
    pub parameters: Option<Value>,
    /// Atomic single-key update merged into the node's current parameters.
    pub parameter_patch: Option<(String, Value)>,
    pub settings: Option<Value>,
    pub delete: bool,
}

#[component]
fn ItemPager(index: usize, total: usize, on_change: EventHandler<usize>) -> Element {
    rsx! {
        div { class: "flex items-center gap-0.5 text-[11px] text-muted-foreground",
            Button {
                size: ButtonSize::IconSm,
                variant: ButtonVariant::Ghost,
                class: Some("size-6".into()),
                disabled: index == 0,
                onclick: move |_| on_change.call(index.saturating_sub(1)),
                ChevronLeft { class: Some("h-3.5 w-3.5".into()) }
            }
            span { class: "min-w-[3.5rem] text-center tabular-nums",
                "{index + 1} / {total}"
            }
            Button {
                size: ButtonSize::IconSm,
                variant: ButtonVariant::Ghost,
                class: Some("size-6".into()),
                disabled: index + 1 >= total,
                onclick: move |_| on_change.call(index + 1),
                ChevronRight { class: Some("h-3.5 w-3.5".into()) }
            }
        }
    }
}

#[component]
fn NodeSettingsTab(
    id: String,
    settings: Value,
    on_update: EventHandler<NodeUpdate>,
) -> Element {
    let disabled = settings
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let always_out = settings
        .get("alwaysOutputData")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let execute_once = settings
        .get("executeOnce")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let retry = settings
        .get("retryOnFail")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_tries = settings
        .get("maxTries")
        .and_then(|v| v.as_u64())
        .unwrap_or(3);
    let wait_ms = settings
        .get("waitBetweenTries")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);
    let on_error = settings
        .get("onError")
        .and_then(|v| v.as_str())
        .unwrap_or("stopWorkflow")
        .to_string();
    let notes = settings
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let display_note = settings
        .get("displayNoteInFlow")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "flex items-center gap-2",
                Switch {
                    checked: always_out,
                    on_checked_change: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |checked| {
                            patch_settings(nid.clone(), settings.clone(), "alwaysOutputData", json!(checked), on_update);
                        }
                    },
                }
                Label { class: Some("text-xs".into()), "Always Output Data" }
            }
            div { class: "flex items-center gap-2",
                Switch {
                    checked: execute_once,
                    on_checked_change: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |checked| {
                            patch_settings(nid.clone(), settings.clone(), "executeOnce", json!(checked), on_update);
                        }
                    },
                }
                Label { class: Some("text-xs".into()), "Execute Once" }
            }
            div { class: "flex items-center gap-2",
                Switch {
                    checked: retry,
                    on_checked_change: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |checked| {
                            patch_settings(nid.clone(), settings.clone(), "retryOnFail", json!(checked), on_update);
                        }
                    },
                }
                Label { class: Some("text-xs".into()), "Retry On Fail" }
            }
            if retry {
                div { class: "flex flex-col gap-1.5",
                    Label { class: Some("text-[11px] text-muted-foreground".into()), "Max Tries" }
                    Input {
                        r#type: Some("number".into()),
                        value: max_tries.to_string(),
                        oninput: {
                            let nid = id.clone();
                            let settings = settings.clone();
                            move |e: FormEvent| {
                                let n: u64 = e.value().parse().unwrap_or(3).max(1);
                                patch_settings(nid.clone(), settings.clone(), "maxTries", json!(n), on_update);
                            }
                        },
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    Label { class: Some("text-[11px] text-muted-foreground".into()), "Wait Between Tries (ms)" }
                    Input {
                        r#type: Some("number".into()),
                        value: wait_ms.to_string(),
                        oninput: {
                            let nid = id.clone();
                            let settings = settings.clone();
                            move |e: FormEvent| {
                                let n: u64 = e.value().parse().unwrap_or(0);
                                patch_settings(nid.clone(), settings.clone(), "waitBetweenTries", json!(n), on_update);
                            }
                        },
                    }
                }
            }
            div { class: "flex items-center gap-2",
                Switch {
                    checked: disabled,
                    on_checked_change: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |checked| {
                            patch_settings(nid.clone(), settings.clone(), "disabled", json!(checked), on_update);
                        }
                    },
                }
                Label { class: Some("text-xs".into()), "Deactivated" }
            }
            div { class: "flex flex-col gap-1.5",
                Label { class: Some("text-[11px] text-muted-foreground".into()), "On Error" }
                Select {
                    value: on_error.clone(),
                    onchange: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |e: FormEvent| {
                            patch_settings(nid.clone(), settings.clone(), "onError", json!(e.value()), on_update);
                        }
                    },
                    SelectOption { value: "stopWorkflow".to_string(), "Stop Workflow" }
                    SelectOption { value: "continue".to_string(), "Continue" }
                    SelectOption {
                        value: "continueErrorOutput".to_string(),
                        "Continue (using error output)"
                    }
                }
            }
            div { class: "flex flex-col gap-1.5",
                Label { class: Some("text-[11px] text-muted-foreground".into()), "Notes" }
                Textarea {
                    class: Some("min-h-[72px] text-xs md:text-xs".into()),
                    value: notes.clone(),
                    oninput: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |e: FormEvent| {
                            patch_settings(nid.clone(), settings.clone(), "notes", json!(e.value()), on_update);
                        }
                    },
                }
            }
            div { class: "flex items-center gap-2",
                Switch {
                    checked: display_note,
                    on_checked_change: {
                        let nid = id.clone();
                        let settings = settings.clone();
                        move |checked| {
                            patch_settings(
                                nid.clone(),
                                settings.clone(),
                                "displayNoteInFlow",
                                json!(checked),
                                on_update,
                            );
                        }
                    },
                }
                Label { class: Some("text-xs".into()), "Display note in flow" }
            }
            div { class: "rounded-md border bg-muted/20 px-2.5 py-1.5 text-[11px] text-muted-foreground",
                "Id "
                code { class: "ml-1 text-foreground", "{id}" }
            }
        }
    }
}

#[component]
pub fn NodeDetailsPanel(
    flow_nodes: Signal<Vec<Node<StudioNodeData>>>,
    doc_nodes: Signal<Vec<Value>>,
    #[props(default)] doc_edges: Signal<Vec<Value>>,
    on_close: EventHandler<()>,
    on_update: EventHandler<NodeUpdate>,
    #[props(default)] workflow_id: Option<String>,
    #[props(default)] workflow_enabled: bool,
    #[props(default)] on_execute_node: Option<EventHandler<String>>,
    #[props(default)] on_pin_sample: Option<EventHandler<(String, Value)>>,
    #[props(default)] on_toast: Option<EventHandler<String>>,
    #[props(default)] run_logs: Signal<Vec<Value>>,
    #[props(default)] node_samples: Signal<std::collections::HashMap<String, Value>>,
    #[props(default)] sample_output: Option<Value>,
    #[props(default)] on_param_error: Option<EventHandler<String>>,
) -> Element {
    let mut expr_bus = use_signal(|| None::<String>);
    let active_field = use_signal(|| None::<String>);
    let mut center_tab = use_signal(|| "parameters".to_string());
    let mut input_index = use_signal(|| 0usize);
    let mut output_index = use_signal(|| 0usize);
    let mut executing = use_signal(|| false);
    let mut webhook_sample = use_signal(|| None::<WebhookSample>);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);

    let selected_id = use_memo(move || {
        flow_nodes()
            .into_iter()
            .find(|n| n.selected)
            .map(|n| n.id.clone())
    });

    use_effect(move || {
        let _ = selected_id();
        center_tab.set("parameters".into());
        input_index.set(0);
        output_index.set(0);
        executing.set(false);
        webhook_sample.set(None);
    });

    use_effect(move || {
        let _ = node_samples();
        executing.set(false);
    });

    let selected_doc = use_memo(move || {
        let id = selected_id()?;
        doc_nodes()
            .into_iter()
            .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
    });

    let selected_flow = use_memo(move || {
        let id = selected_id()?;
        flow_nodes()
            .into_iter()
            .find(|n| n.id == id)
    });

    let dialog_label = selected_flow()
        .map(|f| f.label.clone())
        .or_else(|| {
            selected_doc().and_then(|d| {
                d.get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
        })
        .unwrap_or_else(|| "Node details".into());

    rsx! {
        div { class: "absolute inset-0 z-40", "data-ndv": "1",
            button {
                r#type: "button",
                class: "absolute inset-0 bg-black/45",
                "aria-label": "Close node editor",
                onclick: move |_| on_close.call(()),
            }
            aside {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{dialog_label}",
                // Must match React NDV + classes present in desktop Tailwind
                // (inset-3 / sm:inset-4 / md:inset-5 are not generated → content-sized dialog).
                class: "absolute inset-4 flex flex-col overflow-hidden rounded-xl border bg-background shadow-2xl sm:inset-5 md:inset-6",
                onclick: move |e| e.stop_propagation(),

                if let (Some(doc), Some(flow)) = (selected_doc(), selected_flow()) {
                    {
                        let id = flow.id.clone();
                        let ty = flow
                            .node_type
                            .clone()
                            .or_else(|| doc.get("type").and_then(|v| v.as_str()).map(String::from))
                            .unwrap_or_else(|| "unknown".into());
                        let name = doc
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(flow.label.as_str())
                            .to_string();
                        let parameters = merge_defaults(
                            &ty,
                            doc.get("parameters").unwrap_or(&Value::Object(Default::default())),
                        );
                        let settings = merge_settings(&doc);
                        let schema = get_schema(&ty);
                        let accent = crate::components::flow::node_accent(&ty);
                        let api_sample = if ty == "apiEndpoint" {
                            sample_from_api_endpoint_parameters(&parameters)
                        } else {
                            None
                        };
                        let effective_input = input_for_node(
                            &id,
                            &ty,
                            &parameters,
                            &run_logs(),
                            &node_samples(),
                            &doc_nodes(),
                            &doc_edges(),
                        )
                        .or_else(|| api_sample.clone());
                        let webhook_output = webhook_sample().map(|s| {
                            Value::Array(vec![json!({
                                "headers": s.headers,
                                "query": s.query,
                                "body": s.body,
                                "method": s.method,
                            })])
                        });
                        let effective_sample = webhook_output.or(sample_output.clone());
                        let output_data = output_for_node(
                            &id,
                            &run_logs(),
                            &node_samples(),
                            &effective_sample,
                        );
                        // apiEndpoint: keep schema sample as output preview when no run yet.
                        let effective_output = output_data.or(api_sample);
                        let webhook_path = parameters
                            .get("path")
                            .map(super::json_string)
                            .unwrap_or_default();
                        let webhook_method = parameters
                            .get("method")
                            .and_then(|v| v.as_str())
                            .unwrap_or("POST")
                            .to_string();
                        let input_items = effective_input
                            .as_ref()
                            .map(|d| as_items(d))
                            .unwrap_or_default();
                        let output_items = effective_output
                            .as_ref()
                            .map(|d| as_items(d))
                            .unwrap_or_default();
                        let safe_in = input_index().min(input_items.len().saturating_sub(1));
                        let safe_out = output_index().min(output_items.len().saturating_sub(1));
                        let current_input = input_items.get(safe_in).cloned();
                        let current_output = output_items.get(safe_out).cloned();
                        let is_webhook = ty == "webhook";
                        let can_execute = workflow_id.is_some()
                            && on_execute_node.is_some()
                            && !is_webhook;
                        let input_hint = if ty == "apiEndpoint" {
                            "Add Query Params / Body (Key + Value) like Postman to preview $json.query / $json.body here."
                        } else {
                            "No input data yet. Execute an upstream node, run the workflow, or open a run in Executions."
                        };
                        let output_hint = if is_webhook {
                            "Listen for a test event to map incoming fields."
                        } else if ty == "apiEndpoint" {
                            "Add Query Params / Body values to preview request output for mapping."
                        } else {
                            "Execute step or open a run in Executions."
                        };

                        rsx! {
                            div {
                                class: "flex shrink-0 items-center gap-3 border-b px-4 py-2.5",
                                style: "box-shadow: inset 3px 0 0 var(--brand)",
                                Button {
                                    size: ButtonSize::IconSm,
                                    variant: ButtonVariant::Ghost,
                                    title: Some("Back".into()),
                                    onclick: move |_| on_close.call(()),
                                    ArrowLeft { class: Some("h-4 w-4".into()) }
                                }
                                div {
                                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border-2 bg-card",
                                    style: "border-color:{accent};color:{accent}",
                                    NodeGlyph { ty: ty.clone(), class: Some("size-5".into()) }
                                }
                                div { class: "min-w-0 flex-1",
                                    Input {
                                        class: Some("h-8 border-transparent bg-transparent px-1 text-sm font-semibold shadow-none focus:border-input focus:bg-background".into()),
                                        placeholder: Some("Node name".into()),
                                        value: name.clone(),
                                        oninput: {
                                            let id = id.clone();
                                            move |e: FormEvent| {
                                                on_update.call(NodeUpdate {
                                                    id: id.clone(),
                                                    name: Some(e.value()),
                                                    parameters: None,
                                                    parameter_patch: None,
                                                    settings: None,
                                                    delete: false,
                                                });
                                            }
                                        },
                                    }
                                    div { class: "truncate px-1 font-mono text-[11px] text-muted-foreground", "{ty}" }
                                }
                                div { class: "flex shrink-0 items-center gap-1.5",
                                    if can_execute {
                                        Button {
                                            size: ButtonSize::Sm,
                                            variant: ButtonVariant::Outline,
                                            class: Some("h-8 shrink-0 gap-1 px-2 text-xs".into()),
                                            disabled: executing(),
                                            onclick: {
                                                let id = id.clone();
                                                move |_| {
                                                    if let Some(cb) = on_execute_node.as_ref() {
                                                        executing.set(true);
                                                        cb.call(id.clone());
                                                    }
                                                }
                                            },
                                            Play { class: Some("h-3.5 w-3.5".into()) }
                                            if executing() { "Running…" } else { "Execute step" }
                                        }
                                    }
                                    Button {
                                        size: ButtonSize::Sm,
                                        variant: ButtonVariant::Outline,
                                        class: Some("h-8 gap-1 border-destructive/40 px-2 text-xs text-destructive hover:bg-destructive/10".into()),
                                        title: Some("Delete node".into()),
                                        onclick: {
                                            let id = id.clone();
                                            let name = name.clone();
                                            move |_| {
                                                confirm_delete.set(Some((id.clone(), name.clone())));
                                            }
                                        },
                                        Trash2 { class: Some("h-3.5 w-3.5".into()) }
                                    }
                                    Button {
                                        size: ButtonSize::IconSm,
                                        variant: ButtonVariant::Ghost,
                                        title: Some("Close".into()),
                                        onclick: move |_| on_close.call(()),
                                        X { class: Some("h-4 w-4".into()) }
                                    }
                                }
                            }

                            ResizablePanelGroup {
                                orientation: ResizableOrientation::Horizontal,
                                panel_count: 3,
                                initial_sizes: vec![25.0, 50.0, 25.0],
                                class: Some("min-h-0 flex-1".into()),

                                ResizablePanel {
                                    panel_index: 0,
                                    min_pct: 18.0,
                                    max_pct: 45.0,
                                    class: Some("flex h-full min-h-0 flex-col border-r".into()),
                                    div { class: "flex shrink-0 items-center justify-between border-b px-3 py-2",
                                        h3 { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "Input" }
                                        if input_items.len() > 1 {
                                            ItemPager {
                                                index: safe_in,
                                                total: input_items.len(),
                                                on_change: move |i| input_index.set(i),
                                            }
                                        } else if input_items.len() == 1 {
                                            span { class: "text-[10px] text-muted-foreground", "1 item" }
                                        }
                                    }
                                    div { class: "flex min-h-0 flex-1 flex-col overflow-hidden p-3",
                                        NdvDataPane {
                                            title: "Input data".to_string(),
                                            data: effective_input.clone().or(current_input),
                                            empty_hint: input_hint.to_string(),
                                            on_insert_expr: move |expr: String| expr_bus.set(Some(expr)),
                                        }
                                    }
                                }

                                ResizableHandle { handle_index: 0 }

                                ResizablePanel {
                                    panel_index: 1,
                                    min_pct: 30.0,
                                    max_pct: 70.0,
                                    class: Some("flex h-full min-h-0 flex-col".into()),
                                    div { class: "shrink-0 border-b px-3 pt-2",
                                        div { class: "flex items-center gap-0.5 rounded-md bg-muted/50 p-0.5",
                                            Button {
                                                size: ButtonSize::Sm,
                                                variant: if center_tab() == "parameters" {
                                                    ButtonVariant::Secondary
                                                } else {
                                                    ButtonVariant::Ghost
                                                },
                                                class: Some("h-auto flex-1 rounded px-2 py-1 text-[10px] font-medium".into()),
                                                onclick: move |_| center_tab.set("parameters".into()),
                                                "Parameters"
                                            }
                                            Button {
                                                size: ButtonSize::Sm,
                                                variant: if center_tab() == "settings" {
                                                    ButtonVariant::Secondary
                                                } else {
                                                    ButtonVariant::Ghost
                                                },
                                                class: Some("h-auto flex-1 rounded px-2 py-1 text-[10px] font-medium".into()),
                                                onclick: move |_| center_tab.set("settings".into()),
                                                "Settings"
                                            }
                                        }
                                    }
                                    div { class: "flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto overscroll-contain p-3",
                                        if center_tab() == "parameters" {
                                            if is_webhook {
                                                if let Some(wid) = workflow_id.clone() {
                                                    WebhookListenPanel {
                                                        workflow_id: wid,
                                                        path: webhook_path.clone(),
                                                        method: webhook_method.clone(),
                                                        workflow_enabled,
                                                        sample: webhook_sample(),
                                                        on_toast,
                                                        on_sample: {
                                                            let node_id = id.clone();
                                                            move |s: Option<WebhookSample>| {
                                                                webhook_sample.set(s.clone());
                                                                if let Some(sample) = s {
                                                                    let mapped = json!({
                                                                        "headers": sample.headers,
                                                                        "query": sample.query,
                                                                        "body": sample.body,
                                                                        "method": sample.method,
                                                                    });
                                                                    let out = Value::Array(vec![mapped.clone()]);
                                                                    node_samples.with_mut(|m| {
                                                                        m.insert(node_id.clone(), out);
                                                                    });
                                                                    if let Some(cb) = on_pin_sample.as_ref() {
                                                                        cb.call((node_id.clone(), Value::Array(vec![mapped])));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                    }
                                                }
                                            }
                                            if let Some(schema) = schema {
                                                if schema.fields.is_empty() {
                                                    GenericParamsEditor {
                                                        id: id.clone(),
                                                        node_type: ty.clone(),
                                                        parameters: parameters.clone(),
                                                        on_update,
                                                        on_param_error,
                                                    }
                                                } else {
                                                    for field in schema.fields.iter() {
                                                        if field_visible(field, &parameters) {
                                                            SchemaFieldEditor {
                                                                id: id.clone(),
                                                                node_type: ty.clone(),
                                                                field: field.clone(),
                                                                parameters: parameters.clone(),
                                                                on_update,
                                                                on_param_error,
                                                                expr_bus,
                                                                active_field,
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                GenericParamsEditor {
                                                    id: id.clone(),
                                                    node_type: ty.clone(),
                                                    parameters: parameters.clone(),
                                                    on_update,
                                                    on_param_error,
                                                }
                                            }
                                        } else {
                                            NodeSettingsTab {
                                                id: id.clone(),
                                                settings: settings.clone(),
                                                on_update,
                                            }
                                        }
                                    }
                                }

                                ResizableHandle { handle_index: 1 }

                                ResizablePanel {
                                    panel_index: 2,
                                    min_pct: 18.0,
                                    max_pct: 45.0,
                                    class: Some("flex h-full min-h-0 flex-col border-l".into()),
                                    div { class: "flex shrink-0 items-center justify-between gap-2 border-b px-3 py-2 pr-4",
                                        h3 { class: "min-w-0 truncate text-xs font-semibold uppercase tracking-wide text-muted-foreground", "Output" }
                                        div { class: "flex shrink-0 items-center gap-1.5",
                                            if output_items.len() > 1 {
                                                ItemPager {
                                                    index: safe_out,
                                                    total: output_items.len(),
                                                    on_change: move |i| output_index.set(i),
                                                }
                                            } else if output_items.len() == 1 {
                                                span { class: "text-[10px] text-muted-foreground", "1 item" }
                                            }
                                            if let Some(ref out) = effective_output {
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    variant: ButtonVariant::Outline,
                                                    class: Some("h-auto rounded px-1.5 py-0.5 text-[10px]".into()),
                                                    title: Some("Pin output as sample".into()),
                                                    onclick: {
                                                        let id = id.clone();
                                                        let out = out.clone();
                                                        move |_| {
                                                            if let Some(cb) = on_pin_sample.as_ref() {
                                                                cb.call((id.clone(), out.clone()));
                                                            }
                                                            if let Some(cb) = on_toast.as_ref() {
                                                                cb.call("Output pinned".into());
                                                            }
                                                        }
                                                    },
                                                    "Pin"
                                                }
                                            }
                                        }
                                    }
                                    div { class: "flex min-h-0 flex-1 flex-col overflow-hidden p-3",
                                        NdvDataPane {
                                            title: "Output data".to_string(),
                                            data: effective_output.clone().or(current_output),
                                            empty_hint: output_hint.to_string(),
                                            on_insert_expr: move |expr: String| expr_bus.set(Some(expr)),
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "flex shrink-0 items-center gap-2 border-b px-4 py-2.5",
                        style: "box-shadow: inset 3px 0 0 var(--brand)",
                        Button {
                            size: ButtonSize::IconSm,
                            variant: ButtonVariant::Ghost,
                            title: Some("Back".into()),
                            onclick: move |_| on_close.call(()),
                            ArrowLeft { class: Some("h-4 w-4".into()) }
                        }
                        h2 { class: "flex-1 truncate text-sm font-semibold", "Node details" }
                        Button {
                            size: ButtonSize::IconSm,
                            variant: ButtonVariant::Ghost,
                            title: Some("Close".into()),
                            onclick: move |_| on_close.call(()),
                            X { class: Some("h-4 w-4".into()) }
                        }
                    }
                    div { class: "flex flex-1 items-center justify-center p-6 text-sm text-muted-foreground",
                        "Select a node on the canvas."
                    }
                }
            }

            if let Some((del_id, del_name)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let description = format!(
                        "This will permanently remove node \"{del_name}\" from the workflow. This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete node?".to_string(),
                            description,
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                confirm_delete.set(None);
                                on_update.call(NodeUpdate {
                                    id: del_id_confirm.clone(),
                                    name: None,
                                    parameters: None,
                                    parameter_patch: None,
                                    settings: None,
                                    delete: true,
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SchemaFieldEditor(
    id: String,
    node_type: String,
    field: crate::components::node_schemas::FieldDef,
    parameters: Value,
    on_update: EventHandler<NodeUpdate>,
    on_param_error: Option<EventHandler<String>>,
    expr_bus: Signal<Option<String>>,
    active_field: Signal<Option<String>>,
) -> Element {
    let key = field.key.clone();
    // Dataverse: UI field is getQuery; older seeded flows stored legacy `query`.
    let value = parameters
        .get(&key)
        .cloned()
        .or_else(|| {
            if node_type == "dynamics" && key == "getQuery" {
                parameters.get("query").cloned()
            } else {
                None
            }
        })
        .unwrap_or(Value::Null);
    let show_label = field.kind != "boolean";
    let _ = &node_type;

    let commit = {
        let id = id.clone();
        let key = key.clone();
        let field_kind = field.kind.clone();
        let field_label = field.label.clone();
        move |next: Value| {
            // Validate JSON/number fields before patching so we don't store bad values.
            if field_kind == "json" || field_kind == "jsonSchema" {
                if let Value::String(s) = &next {
                    let raw = s.trim();
                    if !raw.is_empty() {
                        if serde_json::from_str::<Value>(raw).is_err() {
                            if let Some(handler) = on_param_error {
                                handler.call(format!("Invalid JSON in {field_label}"));
                            }
                            return;
                        }
                    }
                }
            }
            if field_kind == "number" {
                match &next {
                    Value::Null | Value::Number(_) => {}
                    Value::String(s) if s.trim().is_empty() => {}
                    Value::String(s) => {
                        if s.trim().parse::<f64>().ok().filter(|n| n.is_finite()).is_none() {
                            if let Some(handler) = on_param_error {
                                handler.call(format!("{field_label} must be a number"));
                            }
                            return;
                        }
                    }
                    _ => {
                        if let Some(handler) = on_param_error {
                            handler.call(format!("{field_label} must be a number"));
                        }
                        return;
                    }
                }
            }
            // Patch one key into the live node doc so Query/Monaco edits can't wipe credentialId.
            emit_parameter_patch(id.clone(), key.clone(), next, on_update);
        }
    };

    let field_key = field.key.clone();
    let row_key = format!("{id}-{field_key}");
    let lang = field
        .language
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_sql_code = field.kind == "code" && matches!(lang.as_str(), "sql" | "tsql");
    let row_class = if is_sql_code {
        "mb-3 flex min-h-0 flex-1 flex-col gap-1.5"
    } else {
        "mb-3 flex shrink-0 flex-col gap-1.5"
    };
    rsx! {
        div {
            class: "{row_class}",
            key: "{row_key}",
            if show_label {
                Label {
                    class: Some("text-[11px] font-normal text-muted-foreground".into()),
                    r#for: Some(row_key.clone()),
                    "{field.label}"
                }
            }
            if let Some(desc) = &field.description {
                p { class: "text-[11px] text-muted-foreground/80", "{desc}" }
            }
            div { class: if is_sql_code { "flex min-h-0 flex-1 flex-col" } else { "contents" },
                FieldControl {
                    field: field.clone(),
                    value: value.clone(),
                    expr_bus: Some(expr_bus),
                    active_field: Some(active_field),
                    on_param_error,
                    id_prefix: Some(id.clone()),
                    on_change: move |next| commit(next),
                }
            }
        }
    }
}

#[component]
fn GenericParamsEditor(
    id: String,
    node_type: String,
    parameters: Value,
    on_update: EventHandler<NodeUpdate>,
    on_param_error: Option<EventHandler<String>>,
) -> Element {
    let entries: Vec<(String, Value)> = parameters
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    rsx! {
        if entries.is_empty() {
            p { class: "mb-3 text-xs text-muted-foreground",
                "No parameters yet. Add a key below or pick a typed schema node."
            }
        }
        for (key, value) in entries.into_iter() {
            {
                let id = id.clone();
                let key_label = key.clone();
                let param_key = key.clone();
                let text = value_as_editable_text(&value);
                rsx! {
                    div { class: "mb-3 flex flex-col gap-1.5",
                        Label { class: Some("text-xs font-medium".into()), "{key_label}" }
                        Textarea {
                            class: Some("min-h-[64px] font-mono text-xs md:text-xs".into()),
                            value: text.clone(),
                            oninput: move |e: FormEvent| {
                                let text = e.value();
                                let next_val = serde_json::from_str::<Value>(&text)
                                    .unwrap_or(Value::String(text));
                                emit_parameter_patch(
                                    id.clone(),
                                    param_key.clone(),
                                    next_val,
                                    on_update,
                                );
                            },
                        }
                    }
                }
            }
        }

        {
            let id = id.clone();
            let node_type = node_type.clone();
            let parameters = parameters.clone();
            rsx! {
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Outline,
                    class: Some("h-8 px-3 text-xs".into()),
                    onclick: move |_| {
                        let mut params = parameters.clone();
                        if let Some(obj) = params.as_object_mut() {
                            let mut i = 1;
                            let mut key = format!("param{i}");
                            while obj.contains_key(&key) {
                                i += 1;
                                key = format!("param{i}");
                            }
                            obj.insert(key, Value::String(String::new()));
                        }
                        emit_parameters_update(
                            id.clone(),
                            &node_type,
                            params,
                            on_update,
                            on_param_error,
                        );
                    },
                    "Add parameter"
                }
            }
        }
    }
}

fn value_as_editable_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}