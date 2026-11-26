use dioxus::prelude::*;
use serde_json::Value;

use crate::components::icons::{ChevronDown, ChevronRight, RefreshCw, Trash2};
use crate::components::panels::{
    format_local_datetime, has_binary_data, json_id, json_string, BinaryViewer,
};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
    Card, CardContent, CardHeader, CardSize, CardTitle, ConfirmDeleteDialog, Empty,
    EmptyDescription, EmptyHeader, EmptyTitle, PageLoader, ResizableHandle, ResizableOrientation,
    ResizablePanel, ResizablePanelGroup,
};

fn run_status_variant(status: &str, failed: bool) -> BadgeVariant {
    if failed || status == "failed" {
        BadgeVariant::Destructive
    } else if status == "completed" || status == "succeeded" {
        BadgeVariant::Success
    } else {
        BadgeVariant::Secondary
    }
}

fn run_error_text(run: &serde_json::Value) -> Option<String> {
    if let Some(e) = run.get("error").and_then(|x| x.as_str()) {
        let t = e.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    run.get("logs")
        .and_then(|l| l.as_array())?
        .iter()
        .find(|log| log.get("status").and_then(|s| s.as_str()) == Some("failed"))
        .and_then(|log| log.get("error").and_then(|e| e.as_str()))
        .map(|e| e.trim())
        .filter(|t| !t.is_empty())
        .map(ToString::to_string)
}

fn parse_time_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp_millis())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3fZ")
                .ok()
                .map(|n| n.and_utc().timestamp_millis())
        })
}

fn log_duration_ms(log: &serde_json::Value) -> i64 {
    let started = log.get("startedAt").and_then(|x| x.as_str()).unwrap_or("");
    let finished = log.get("finishedAt").and_then(|x| x.as_str()).unwrap_or("");
    match (parse_time_ms(started), parse_time_ms(finished)) {
        (Some(s), Some(f)) => (f - s).max(0),
        _ => 0,
    }
}

fn run_duration_ms(run: &serde_json::Value) -> Option<i64> {
    let started = run.get("startedAt").and_then(|x| x.as_str()).unwrap_or("");
    let finished = run.get("finishedAt").and_then(|x| x.as_str()).unwrap_or("");
    match (parse_time_ms(started), parse_time_ms(finished)) {
        (Some(s), Some(f)) if f >= s => Some(f - s),
        _ => None,
    }
}

fn format_duration_ms(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 10_000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn json_pretty(value: Option<&serde_json::Value>) -> String {
    match value {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(v) => serde_json::to_string_pretty(v).unwrap_or_else(|_| json_string(v)),
    }
}

#[component]
fn JsonBlock(value: Option<serde_json::Value>) -> Element {
    if value.is_none() || matches!(value.as_ref(), Some(serde_json::Value::Null)) {
        return rsx! {
            span { class: "text-muted-foreground", "—" }
        };
    }
    let text = json_pretty(value.as_ref());
    rsx! {
        pre {
            class: "max-h-40 overflow-auto rounded-md border bg-muted/30 p-2 font-mono text-[11px] whitespace-pre-wrap break-words",
            "{text}"
        }
    }
}

#[component]
fn StepRow(log: serde_json::Value) -> Element {
    let step_status = log.get("status").and_then(|s| s.as_str()).unwrap_or("");
    let failed = step_status == "failed";
    let mut open = use_signal(|| true);
    let node = log.get("nodeName").map(json_string).unwrap_or_default();
    let error = log
        .get("error")
        .and_then(|e| e.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    let duration = format_duration_ms(log_duration_ms(&log));
    let step_variant = run_status_variant(step_status, failed);
    let row_class = if failed {
        "rounded border border-destructive/40 bg-destructive/5"
    } else {
        "rounded border bg-background/80"
    };
    let input = log.get("input").cloned();
    let output = log.get("output").cloned();

    rsx! {
        div { class: "{row_class}",
            button {
                class: "flex w-full items-center gap-2 px-2 py-1.5 text-left text-xs",
                onclick: move |_| open.set(!open()),
                span { class: "font-medium", "{node}" }
                Badge { variant: step_variant, "{step_status}" }
                span { class: "text-muted-foreground", "{duration}" }
                span { class: "ml-auto text-muted-foreground",
                    if open() {
                        ChevronDown { class: Some("size-3.5".into()) }
                    } else {
                        ChevronRight { class: Some("size-3.5".into()) }
                    }
                }
            }
            if let Some(err) = error.clone() {
                if !open() {
                    div {
                        class: "border-t border-destructive/20 px-2 pb-1.5 text-[11px] leading-snug text-destructive line-clamp-2",
                        "{err}"
                    }
                }
            }
            if open() {
                div { class: "flex flex-col gap-2 border-t px-2 py-2",
                    if let Some(err) = error {
                        Alert {
                            variant: AlertVariant::Destructive,
                            class: Some("border-destructive/30 bg-destructive/10 px-2 py-1.5 text-xs".into()),
                            AlertDescription {
                                class: Some("text-destructive whitespace-pre-wrap break-words".into()),
                                "{err}"
                            }
                        }
                    }
                    div {
                        div { class: "mb-1 text-[10px] uppercase text-muted-foreground", "Input" }
                        JsonBlock { value: input.clone() }
                        if input.as_ref().map(has_binary_data).unwrap_or(false) {
                            BinaryViewer { data: input.clone().unwrap_or(Value::Null) }
                        }
                    }
                    div {
                        div { class: "mb-1 text-[10px] uppercase text-muted-foreground", "Output" }
                        JsonBlock { value: output.clone() }
                        if output.as_ref().map(has_binary_data).unwrap_or(false) {
                            BinaryViewer { data: output.clone().unwrap_or(Value::Null) }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn RunsPanel(
    #[props(default)] refresh_key: u32,
    workflow_id: Option<String>,
    #[props(default)] list_only: bool,
    selected_run_id: Option<String>,
    on_select_run: Option<EventHandler<Option<serde_json::Value>>>,
) -> Element {
    let mut runs = use_signal(Vec::<serde_json::Value>::new);
    let mut detail = use_signal(|| None::<serde_json::Value>);
    let mut err = use_signal(|| None::<String>);
    let mut clearing = use_signal(|| false);
    let mut confirm_clear = use_signal(|| false);
    let mut loading = use_signal(|| true);
    let mut detail_loading = use_signal(|| false);
    let mut internal_selected = use_signal(|| None::<String>);
    let mut list_epoch = use_signal(|| 0u32);
    let mut list_request_id = use_signal(|| 0u32);
    // Request id must NOT be read via signal() inside the effect body — that
    // re-subscribes and re-fires forever (stuck "Loading run…").
    let mut detail_request_id = use_signal(|| 0u32);
    let use_internal_selection = selected_run_id.is_none();
    let workflow_id_clear = workflow_id.clone();

    // Props aren't reactive — mirror into signals so effects re-run after Execute / nav.
    let mut refresh_sig = use_signal(|| refresh_key);
    if *refresh_sig.peek() != refresh_key {
        refresh_sig.set(refresh_key);
    }
    let mut workflow_id_sig = use_signal(|| workflow_id.clone());
    if *workflow_id_sig.peek() != workflow_id {
        workflow_id_sig.set(workflow_id.clone());
    }
    let mut selected_run_external = use_signal(|| selected_run_id.clone());
    if *selected_run_external.peek() != selected_run_id {
        selected_run_external.set(selected_run_id.clone());
    }

    let selected_id = selected_run_external().or_else(|| internal_selected());

    use_effect(move || {
        let _ = refresh_sig();
        let _ = workflow_id_sig();
        let _ = list_epoch();
        let mut req = 0u32;
        list_request_id.with_mut(|id| {
            *id += 1;
            req = *id;
        });
        let wf = workflow_id_sig();
        let select_newest = use_internal_selection;
        loading.set(true);
        spawn(async move {
            let mut path = format!("/api/runs?limit=40&_={req}");
            if let Some(id) = wf {
                path.push_str(&format!("&workflowId={}", urlencoding_placeholder(&id)));
            }
            let result = crate::api::get_json(&path).await;
            if list_request_id() != req {
                return;
            }
            match result {
                Ok(v) => {
                    let list = v
                        .get("runs")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let newest_id = list.first().map(json_id);
                    runs.set(list);
                    // Keep step history on the latest run when Execute bumps refresh_key.
                    if select_newest {
                        if let Some(id) = newest_id {
                            internal_selected.set(Some(id));
                        }
                    }
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    });

    use_effect(move || {
        let _ = refresh_sig();
        let _ = list_epoch();
        let id = selected_run_external().or_else(|| internal_selected());
        let mut req = 0u32;
        detail_request_id.with_mut(|n| {
            *n += 1;
            req = *n;
        });
        spawn(async move {
            match id {
                None => {
                    if detail_request_id() != req {
                        return;
                    }
                    detail.set(None);
                    detail_loading.set(false);
                }
                Some(rid) => {
                    detail_loading.set(true);
                    let path = format!("/api/runs/{rid}?_={req}");
                    let result = crate::api::get_json(&path).await;
                    if detail_request_id() != req {
                        return;
                    }
                    match result {
                        Ok(v) => detail.set(v.get("run").cloned()),
                        Err(_) => detail.set(None),
                    }
                    detail_loading.set(false);
                }
            }
        });
    });

    let list = rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-2",
            div { class: "flex items-center justify-between gap-2",
                h2 { class: "text-sm font-semibold", "Runs" }
                div { class: "flex items-center gap-0.5",
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: Some("h-7 gap-1.5 px-2 text-xs text-muted-foreground".into()),
                        disabled: loading(),
                        onclick: move |_| {
                            if loading() {
                                return;
                            }
                            // Bump epoch so the list effect re-fetches immediately.
                            list_epoch.with_mut(|e| *e += 1);
                        },
                        RefreshCw {
                            class: Some(if loading() {
                                "size-3.5 animate-spin".into()
                            } else {
                                "size-3.5".into()
                            })
                        }
                        "Refresh"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: Some("h-7 gap-1.5 px-2 text-xs text-muted-foreground".into()),
                        disabled: runs().is_empty() || clearing(),
                        onclick: move |_| {
                            if runs().is_empty() || clearing() {
                                return;
                            }
                            confirm_clear.set(true);
                        },
                        Trash2 { class: Some("size-3.5".into()) }
                        "Clear"
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "flex flex-col gap-2",
                    if loading() {
                        PageLoader { label: "Loading runs…", class: Some("min-h-[8rem]") }
                    } else {
                    for r in runs().iter().cloned() {
                        {
                            let id = json_id(&r);
                            let wf_name = r.get("workflowName").map(json_string).unwrap_or_default();
                            let status = r.get("status").map(json_string).unwrap_or_default();
                            let trigger = r.get("trigger").map(json_string).unwrap_or_default();
                            let started = format_local_datetime(
                                &r.get("startedAt").map(json_string).unwrap_or_default(),
                            );
                            let duration = run_duration_ms(&r).map(format_duration_ms);
                            let err_preview = run_error_text(&r);
                            let failed = status == "failed" || err_preview.is_some();
                            let selected = selected_id.as_deref() == Some(id.as_str());
                            let select_id = id.clone();
                            let cb = on_select_run.clone();
                            let internal = use_internal_selection;
                            let row_class = if selected {
                                "rounded-md border bg-accent p-2 text-left text-sm"
                            } else if failed {
                                "rounded-md border border-destructive/40 bg-destructive/5 p-2 text-left text-sm hover:bg-destructive/10"
                            } else {
                                "rounded-md border bg-card p-2 text-left text-sm hover:bg-muted/50"
                            };
                            let badge_variant = run_status_variant(&status, failed);
                            rsx! {
                                button {
                                    key: "{id}",
                                    class: "{row_class}",
                                    onclick: move |_| {
                                        if internal {
                                            // Detail fetch is owned by the selection effect.
                                            internal_selected.set(Some(select_id.clone()));
                                        }
                                        if let Some(cb) = cb.clone() {
                                            let path = format!("/api/runs/{select_id}");
                                            spawn(async move {
                                                if let Ok(v) = crate::api::get_json(&path).await {
                                                    cb.call(v.get("run").cloned());
                                                }
                                            });
                                        }
                                    },
                                    div { class: "flex flex-wrap items-center gap-2",
                                        span { class: "font-medium", "{wf_name}" }
                                        Badge { variant: badge_variant, "{status}" }
                                        if let Some(dur) = duration.clone() {
                                            Badge { variant: BadgeVariant::Outline, "{dur}" }
                                        }
                                        span { class: "text-xs text-muted-foreground", "{trigger}" }
                                    }
                                    div { class: "text-xs text-muted-foreground", "{started}" }
                                    if let Some(e) = err_preview {
                                        div {
                                            class: "mt-1 text-xs leading-snug text-destructive line-clamp-3 whitespace-pre-wrap break-words",
                                            "{e}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if runs().is_empty() {
                        Empty {
                            class: Some("border-dashed p-6".into()),
                            EmptyHeader {
                                EmptyTitle { class: Some("text-sm".into()), "No runs yet" }
                            }
                        }
                    }
                    }
                }
            }
        }
    };

    let clear_dialog = rsx! {
        if confirm_clear() {
            ConfirmDeleteDialog {
                title: "Clear runs?".to_string(),
                description: "This will permanently delete all listed runs. This action cannot be undone.".to_string(),
                confirm_label: "Clear".to_string(),
                busy: clearing(),
                on_cancel: move |_| confirm_clear.set(false),
                on_confirm: move |_| {
                    clearing.set(true);
                    let wf = workflow_id_clear.clone();
                    let cb = on_select_run.clone();
                    let internal = use_internal_selection;
                    spawn(async move {
                        let mut path = "/api/runs".to_string();
                        if let Some(id) = wf {
                            path.push_str(&format!("?workflowId={}", urlencoding_placeholder(&id)));
                        }
                        if crate::api::delete_json(&path).await.is_ok() {
                            runs.set(vec![]);
                            detail.set(None);
                            if internal {
                                internal_selected.set(None);
                            }
                            if let Some(cb) = cb {
                                cb.call(None);
                            }
                            confirm_clear.set(false);
                        }
                        clearing.set(false);
                    });
                },
            }
        }
    };

    if list_only {
        return rsx! {
            div { class: "relative h-full min-h-0 p-3",
                {list}
                {clear_dialog}
            }
        };
    }

    rsx! {
        div { class: "relative h-full min-h-0",
        ResizablePanelGroup {
            orientation: ResizableOrientation::Horizontal,
            initial_sizes: vec![50.0, 50.0],
            class: Some("h-full min-h-0 gap-0 p-3".into()),
            ResizablePanel {
                panel_index: 0,
                min_pct: 25.0,
                max_pct: 75.0,
                class: Some("h-full min-h-0 pr-2".into()),
                {list}
            }
            ResizableHandle { handle_index: 0, container_px: 900.0 }
            ResizablePanel {
                panel_index: 1,
                min_pct: 25.0,
                max_pct: 75.0,
                class: Some("flex h-full min-h-0 flex-col".into()),
                Card {
                    size: CardSize::Sm,
                    class: Some("flex h-full min-h-0 flex-col gap-2 py-2".into()),
                    CardHeader {
                        class: Some("px-2 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()), "Step history" }
                    }
                    CardContent {
                        class: Some("flex min-h-0 flex-1 flex-col overflow-hidden px-2".into()),
                if detail_loading() {
                    PageLoader { label: "Loading run…", class: Some("min-h-[8rem]") }
                } else if detail().is_none() {
                    Empty {
                        class: Some("border-0 p-4".into()),
                        EmptyHeader {
                            EmptyDescription {
                                "Select a run to inspect each step's inputs, outputs, and failures."
                            }
                        }
                    }
                } else if let Some(d) = detail() {
                    {
                        let wf = d.get("workflowName").map(json_string).unwrap_or_default();
                        let status = d.get("status").map(json_string).unwrap_or_default();
                        let trigger = d.get("trigger").map(json_string).unwrap_or_default();
                        let actor = d.get("actor").map(json_string).unwrap_or_default();
                        let time_saved = d
                            .get("timeSavedMinutes")
                            .and_then(|x| x.as_f64())
                            .filter(|&m| m > 0.0)
                            .map(|m| format!(" · saved {} min", m.round() as i64))
                            .unwrap_or_default();
                        let detail_error = run_error_text(&d);
                        let mut meta = format!("{wf} · {status}");
                        if !trigger.is_empty() {
                            meta.push_str(&format!(" · {trigger}"));
                        }
                        if !actor.is_empty() {
                            meta.push_str(&format!(" · {actor}"));
                        }
                        meta.push_str(&time_saved);
                        rsx! {
                            div { class: "min-h-0 flex-1 overflow-y-auto",
                                div { class: "flex flex-col gap-2 pr-2",
                                    div { class: "text-xs text-muted-foreground", "{meta}" }
                                    if let Some(e) = detail_error {
                                        Alert {
                                            variant: AlertVariant::Destructive,
                                            class: Some("bg-destructive/10 px-2 py-1.5 text-xs".into()),
                                            AlertDescription {
                                                class: Some("text-destructive whitespace-pre-wrap break-words".into()),
                                                "{e}"
                                            }
                                        }
                                    }
                                    if let Some(logs) = d.get("logs").and_then(|l| l.as_array()) {
                                        for log in logs.iter().cloned() {
                                            {
                                                let key = format!(
                                                    "{}-{}",
                                                    log.get("nodeId").map(json_string).unwrap_or_default(),
                                                    log.get("startedAt").map(json_string).unwrap_or_default()
                                                );
                                                rsx! {
                                                    StepRow { key: "{key}", log }
                                                }
                                            }
                                        }
                                    }
                                    if d.get("logs").and_then(|l| l.as_array()).map(|a| a.is_empty()).unwrap_or(true) {
                                        Empty {
                                            class: Some("border-0 p-4".into()),
                                            EmptyHeader {
                                                EmptyDescription { "No step logs" }
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
            }
        }
        {clear_dialog}
        }
    }
}

fn urlencoding_placeholder(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}