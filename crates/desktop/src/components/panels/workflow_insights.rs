use dioxus::prelude::*;

use crate::components::icons::{ArrowDown, ArrowUp, RefreshCw};
use crate::components::panels::json_string;
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, AreaChart, Badge, BadgeVariant, Button, ButtonSize,
    ButtonVariant, Card, CardContent, CardSize, ChartCurve, DataTable, DataTableBody, DataTableCell,
    DataTableHead, DataTableHeader, DataTableRow, DataTableWrapper, Empty, EmptyDescription,
    EmptyHeader, SvgAreaChart,
};

fn fmt_count(n: f64) -> String {
    let v = n.round() as i64;
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let formatted: String = out.chars().rev().collect();
    if v < 0 {
        format!("-{formatted}")
    } else {
        formatted
    }
}

fn fmt_ratio(n: f64) -> String {
    format!("{:.1}%", n * 100.0)
}

fn fmt_ms(n: f64) -> String {
    if n < 1000.0 {
        format!("{}ms", n.round() as i64)
    } else {
        format!("{:.2}s", n / 1000.0)
    }
}

fn format_range(from: &str, to: &str) -> String {
    let fmt = |s: &str| -> String {
        chrono::NaiveDate::parse_from_str(&s[..s.len().min(10)], "%Y-%m-%d")
            .map(|d| d.format("%b %d").to_string())
            .unwrap_or_else(|_| s.to_string())
    };
    format!("{} – {}", fmt(from), fmt(to))
}

fn metric_value(insights: &serde_json::Value, key: &str) -> (f64, Option<f64>) {
    let cur = insights
        .get(key)
        .and_then(|m| m.get("value"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let prev = insights
        .get(key)
        .and_then(|m| m.get("previous"))
        .and_then(|v| v.as_f64());
    (cur, prev)
}

fn delta_pct(current: f64, previous: Option<f64>) -> Option<(f64, bool)> {
    let previous = previous?;
    if previous.abs() < 1e-9 {
        return None;
    }
    let pct = ((current - previous) / previous) * 100.0;
    Some((pct, pct >= 0.0))
}

fn series_f64(point: &serde_json::Value, key: &str) -> f64 {
    point.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn chart_points(data: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    // Keep empty buckets after the first hit so gaps stay visible, but drop a
    // long empty prefix (and trailing empties) that flatten ApexCharts.
    let first = data
        .iter()
        .position(|d| series_f64(d, "total") > 0.0);
    let last = data
        .iter()
        .rposition(|d| series_f64(d, "total") > 0.0);
    match (first, last) {
        (Some(a), Some(b)) if a <= b => data[a..=b].iter().collect(),
        _ => Vec::new(),
    }
}

fn chart_labels(data: &[&serde_json::Value]) -> String {
    let labels: Vec<String> = data
        .iter()
        .map(|d| d.get("date").map(json_string).unwrap_or_default())
        .collect();
    serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into())
}

fn traffic_chart_values(data: &[&serde_json::Value]) -> String {
    let success: Vec<f64> = data
        .iter()
        .map(|d| {
            let total = series_f64(d, "total");
            let failed = series_f64(d, "failed").clamp(0.0, total);
            (total - failed).max(0.0)
        })
        .collect();
    let failed: Vec<f64> = data.iter().map(|d| series_f64(d, "failed")).collect();
    serde_json::to_string(&[success, failed]).unwrap_or_else(|_| "[]".into())
}

fn duration_series(data: &[&serde_json::Value]) -> (Vec<String>, Vec<f64>) {
    // Avg request duration (ms) per bucket — same axis as Traffic, values match table Duration.
    let labels: Vec<String> = data
        .iter()
        .map(|d| d.get("date").map(json_string).unwrap_or_default())
        .collect();
    let values: Vec<f64> = data
        .iter()
        .map(|d| {
            if series_f64(d, "total") <= 0.0 {
                0.0
            } else {
                series_f64(d, "averageRunTimeMs").round()
            }
        })
        .collect();
    (labels, values)
}

#[component]
fn KpiCard(
    label: String,
    value: String,
    hint: String,
    delta: Option<(f64, bool)>,
    invert_good: bool,
) -> Element {
    let delta_el = delta.map(|(pct, up)| {
        let good = if invert_good { !up } else { up };
        let class = if pct.abs() < 0.05 {
            "text-muted-foreground"
        } else if good {
            "text-primary"
        } else {
            "text-destructive"
        };
        let text = format!("{:.1}%", pct.abs());
        (class, up, text)
    });

    rsx! {
        Card {
            size: CardSize::Sm,
            class: Some("px-3 py-2.5 gap-0".into()),
            div { class: "text-xs font-medium text-muted-foreground", "{label}" }
            div { class: "mt-1.5 flex items-end justify-between gap-2",
                div { class: "text-xl font-semibold tabular-nums tracking-tight", "{value}" }
                if let Some((class, up, text)) = delta_el {
                    span { class: "inline-flex items-center gap-0.5 text-[11px] tabular-nums {class}",
                        if up {
                            ArrowUp { class: Some("h-3 w-3".into()) }
                        } else {
                            ArrowDown { class: Some("h-3 w-3".into()) }
                        }
                        "{text}"
                    }
                }
            }
            div { class: "mt-1 text-[10px] text-muted-foreground", "{hint}" }
        }
    }
}

fn status_badge(status: &str) -> (BadgeVariant, String) {
    match status {
        "completed" | "success" | "succeeded" => (BadgeVariant::Success, "completed".into()),
        "failed" | "error" => (BadgeVariant::Destructive, "failed".into()),
        "running" | "queued" | "waiting" | "retrying" => {
            (BadgeVariant::Secondary, status.to_string())
        }
        other => (BadgeVariant::Outline, other.to_string()),
    }
}

/// Cloudflare-style API performance insights for a single workflow.
#[component]
pub fn WorkflowApiInsights(
    workflow_id: String,
    #[props(default)] refresh_key: u32,
) -> Element {
    let mut insights = use_signal(|| None::<serde_json::Value>);
    let mut days = use_signal(|| 7_i32);
    let mut err = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    let mut fetch_epoch = use_signal(|| 0_u32);

    let mut refresh_sig = use_signal(|| refresh_key);
    if *refresh_sig.peek() != refresh_key {
        refresh_sig.set(refresh_key);
    }
    let mut workflow_sig = use_signal(|| workflow_id.clone());
    if *workflow_sig.peek() != workflow_id {
        workflow_sig.set(workflow_id.clone());
    }

    use_effect(move || {
        let _ = refresh_sig();
        let _ = days();
        let _ = fetch_epoch();
        let wf = workflow_sig();
        let epoch = fetch_epoch();
        let d = days();
        loading.set(true);
        spawn(async move {
            crate::chart_assets::ensure_charts_loaded().await;
            let path = format!("/api/insights?days={d}&workflowId={wf}&_={epoch}");
            let result = crate::api::get_json(&path).await;
            if fetch_epoch() != epoch {
                return;
            }
            match result {
                Ok(v) => {
                    insights.set(v.get("insights").cloned());
                    err.set(None);
                }
                Err(e) => {
                    insights.set(None);
                    let msg = if e == "Not found" || e == "Failed to fetch" {
                        "Insights API unavailable. Is the BotFlow server running?".into()
                    } else {
                        e
                    };
                    err.set(Some(msg));
                }
            }
            loading.set(false);
            let _ = document::eval(
                "if (window.__bfRefreshCharts) window.__bfRefreshCharts(); return true;",
            )
            .await;
        });
    });

    let by_time = insights()
        .as_ref()
        .and_then(|i| i.get("byTime"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let executions = insights()
        .as_ref()
        .and_then(|i| i.get("executions"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let range_chip = insights().as_ref().map(|i| {
        let from = i.get("from").map(json_string).unwrap_or_default();
        let to = i.get("to").map(json_string).unwrap_or_default();
        format_range(&from, &to)
    });

    let (total, total_prev) = insights()
        .as_ref()
        .map(|i| metric_value(i, "total"))
        .unwrap_or((0.0, None));
    let (success, success_prev) = insights()
        .as_ref()
        .map(|i| metric_value(i, "success"))
        .unwrap_or((0.0, None));
    let (failed, _) = insights()
        .as_ref()
        .map(|i| metric_value(i, "failed"))
        .unwrap_or((0.0, None));
    let (failure_rate, failure_prev) = insights()
        .as_ref()
        .map(|i| metric_value(i, "failureRate"))
        .unwrap_or((0.0, None));
    let (avg_ms, avg_prev) = insights()
        .as_ref()
        .map(|i| metric_value(i, "averageRunTime"))
        .unwrap_or((0.0, None));

    let blank = if loading() { "…" } else { "—" };
    let has_data = insights().is_some();
    let total_display = if has_data {
        fmt_count(total)
    } else {
        blank.to_string()
    };
    let error_display = if has_data {
        fmt_ratio(failure_rate)
    } else {
        blank.to_string()
    };
    let avg_display = if has_data {
        fmt_ms(avg_ms)
    } else {
        blank.to_string()
    };
    let success_rate = if total > 0.0 {
        (success / total) * 100.0
    } else {
        0.0
    };
    let success_rate_display = if has_data {
        format!("{success_rate:.1}%")
    } else {
        blank.to_string()
    };

    let chart_data = chart_points(&by_time);
    let traffic_values = traffic_chart_values(&chart_data);
    let traffic_labels = chart_labels(&chart_data);
    let traffic_series =
        serde_json::to_string(&["Success", "Errors"]).unwrap_or_else(|_| "[]".into());
    let (duration_labels, duration_values) = duration_series(&chart_data);

    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-3 overflow-y-auto bg-background p-4",
            div { class: "flex flex-wrap items-center gap-2",
                div {
                    h2 { class: "text-base font-semibold tracking-tight", "API Insights" }
                    p { class: "text-xs text-muted-foreground",
                        "Request volume, latency, and error rate for this endpoint"
                    }
                }
                div { class: "flex-1" }
                if let Some(chip) = range_chip {
                    span { class: "rounded-md border bg-card px-2.5 py-1 text-xs text-muted-foreground", "{chip}" }
                }
                div { class: "flex overflow-hidden rounded-md border",
                    for d in [7_i32, 14, 30] {
                        Button {
                            key: "{d}",
                            variant: if days() == d {
                                ButtonVariant::Default
                            } else {
                                ButtonVariant::Ghost
                            },
                            size: ButtonSize::Sm,
                            class: Some("rounded-none px-2.5".into()),
                            onclick: move |_| days.set(d),
                            "{d}d"
                        }
                    }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    disabled: loading(),
                    onclick: move |_| {
                        if loading() {
                            return;
                        }
                        fetch_epoch.with_mut(|e| *e += 1);
                    },
                    RefreshCw { class: Some(format!("h-3.5 w-3.5{}", if loading() { " animate-spin" } else { "" })) }
                    "Refresh"
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("bg-destructive/10".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            div { class: "grid grid-cols-2 gap-2 xl:grid-cols-4",
                KpiCard {
                    label: "Total requests".to_string(),
                    value: total_display,
                    hint: "All executions in range".to_string(),
                    delta: delta_pct(total, total_prev),
                    invert_good: false,
                }
                KpiCard {
                    label: "Success rate".to_string(),
                    value: success_rate_display,
                    hint: format!("{} succeeded", fmt_count(success)),
                    delta: delta_pct(success, success_prev),
                    invert_good: false,
                }
                KpiCard {
                    label: "Error rate".to_string(),
                    value: error_display,
                    hint: format!("{} failed", fmt_count(failed)),
                    delta: delta_pct(failure_rate, failure_prev),
                    invert_good: true,
                }
                KpiCard {
                    label: "Avg latency".to_string(),
                    value: avg_display,
                    hint: "Mean response time".to_string(),
                    delta: delta_pct(avg_ms, avg_prev),
                    invert_good: true,
                }
            }

            // Left: Recent requests · Right: Traffic / Outcomes / Latency
            div { class: "grid min-h-0 flex-1 gap-3 lg:grid-cols-2",
                Card {
                    class: Some("min-h-0 gap-0 overflow-hidden p-0".into()),
                    CardContent {
                        class: Some("flex h-full min-h-0 flex-col space-y-2 p-3".into()),
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "text-sm font-semibold", "Recent requests" }
                            span { class: "text-[11px] text-muted-foreground",
                                "{executions.len()} shown"
                            }
                        }
                        DataTableWrapper { class: Some("min-h-0 flex-1 max-h-none".into()),
                            DataTable { class: Some("min-w-full".into()),
                                DataTableHeader { class: Some("bg-muted/40 text-xs".into()),
                                    DataTableRow {
                                        DataTableHead { class: Some("px-3 py-2 w-[34%]".into()), "Time" }
                                        DataTableHead { class: Some("px-3 py-2 w-[18%]".into()), "Status" }
                                        DataTableHead { class: Some("px-3 py-2 w-[22%]".into()), "Trigger" }
                                        DataTableHead { class: Some("px-3 py-2 w-[26%] text-right".into()), "Duration" }
                                    }
                                }
                                DataTableBody {
                                    for row in executions.iter() {
                                        {
                                            let at = row.get("at").map(json_string).unwrap_or_default();
                                            let status = row.get("status").map(json_string).unwrap_or_default();
                                            let trigger = row.get("trigger").map(json_string).unwrap_or_else(|| "manual".into());
                                            let duration = row
                                                .get("durationMs")
                                                .and_then(|v| v.as_f64())
                                                .map(fmt_ms)
                                                .unwrap_or_else(|| "—".into());
                                            let local = crate::components::panels::format_local_datetime_short(&at);
                                            let (badge_variant, badge_label) = status_badge(&status);
                                            let key = row.get("id").map(json_string).unwrap_or_else(|| at.clone());
                                            rsx! {
                                                DataTableRow {
                                                    key: "{key}",
                                                    DataTableCell {
                                                        class: Some("px-3 py-2 text-xs tabular-nums text-muted-foreground".into()),
                                                        "{local}"
                                                    }
                                                    DataTableCell { class: Some("px-3 py-2".into()),
                                                        Badge {
                                                            variant: badge_variant,
                                                            class: Some("capitalize text-[10px]".into()),
                                                            "{badge_label}"
                                                        }
                                                    }
                                                    DataTableCell {
                                                        class: Some("px-3 py-2 text-xs capitalize".into()),
                                                        "{trigger}"
                                                    }
                                                    DataTableCell {
                                                        class: Some("px-3 py-2 text-right text-xs tabular-nums font-medium".into()),
                                                        "{duration}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !loading() && insights().is_some() && executions.is_empty() {
                                        DataTableRow {
                                            DataTableCell {
                                                colspan: Some(4),
                                                class: Some("p-0".into()),
                                                Empty {
                                                    class: Some("border-0 py-8".into()),
                                                    EmptyHeader {
                                                        EmptyDescription {
                                                            "No executions yet. Run this workflow or hit the public API to populate insights."
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if loading() && executions.is_empty() {
                                        DataTableRow {
                                            DataTableCell {
                                                colspan: Some(4),
                                                class: Some("px-3 py-6 text-center text-xs text-muted-foreground".into()),
                                                "Loading requests…"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "flex min-h-0 flex-col gap-3",
                    Card {
                        class: Some("gap-0 overflow-hidden p-0".into()),
                        CardContent {
                            class: Some("space-y-3 p-3".into()),
                            div {
                                h3 { class: "text-sm font-semibold", "Traffic" }
                                p { class: "text-[11px] text-muted-foreground", "Successful vs failed requests over time" }
                            }
                            if chart_data.is_empty() {
                                Empty {
                                    class: Some("h-[220px] border-0".into()),
                                    EmptyHeader {
                                        EmptyDescription {
                                            if loading() { "Loading traffic…" } else { "No requests in this window" }
                                        }
                                    }
                                }
                            } else {
                                AreaChart {
                                    class: Some("h-[220px] w-full".into()),
                                    json_values: traffic_values,
                                    json_labels: traffic_labels,
                                    series_names: Some(traffic_series),
                                    stack_type: Some("normal".into()),
                                    curve: ChartCurve::Smooth,
                                    gradient: Some(true),
                                    show_grid: Some(true),
                                    show_yaxis: Some(true),
                                }
                            }
                        }
                    }
                    Card {
                        class: Some("gap-0 overflow-hidden p-0".into()),
                        CardContent {
                            class: Some("space-y-3 p-3".into()),
                            div {
                                h3 { class: "text-sm font-semibold", "Duration" }
                                p { class: "text-[11px] text-muted-foreground", "Average request duration over time" }
                            }
                            if chart_data.is_empty() {
                                Empty {
                                    class: Some("h-[220px] border-0".into()),
                                    EmptyHeader {
                                        EmptyDescription {
                                            if loading() { "Loading duration…" } else { "No duration data in this window" }
                                        }
                                    }
                                }
                            } else {
                                // SVG (not Apex) — second ApexCharts instance stays blank in the desktop webview.
                                SvgAreaChart {
                                    labels: duration_labels,
                                    values: duration_values,
                                    series_name: "Duration".to_string(),
                                    value_suffix: Some("ms".into()),
                                    class: Some("h-[220px] w-full".into()),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
