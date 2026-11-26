use dioxus::prelude::*;

use crate::components::icons::{ArrowDown, ArrowUp, ChevronRight, RefreshCw};
use crate::components::panels::json_string;
use crate::components::ui::{Button, ButtonSize, ButtonVariant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum MetricId {
    Total,
    Failed,
    FailureRate,
    TimeSaved,
    AverageRunTime,
}

impl MetricId {
    fn key(self) -> &'static str {
        match self {
            Self::Total => "total",
            Self::Failed => "failed",
            Self::FailureRate => "failureRate",
            Self::TimeSaved => "timeSaved",
            Self::AverageRunTime => "averageRunTime",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Total => "Prod. executions",
            Self::Failed => "Failed prod. executions",
            Self::FailureRate => "Failure rate",
            Self::TimeSaved => "Time saved",
            Self::AverageRunTime => "Run time (avg.)",
        }
    }

    fn chart_unit(self) -> &'static str {
        match self {
            Self::Total | Self::Failed => "day",
            Self::FailureRate => "%",
            Self::TimeSaved => "h",
            Self::AverageRunTime => "s",
        }
    }

    fn sort_key(self) -> SortKey {
        match self {
            Self::Total => SortKey::Total,
            Self::Failed => SortKey::Failed,
            Self::FailureRate => SortKey::FailureRate,
            Self::TimeSaved => SortKey::TimeSavedMinutes,
            Self::AverageRunTime => SortKey::AverageRunTimeMs,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    WorkflowName,
    Total,
    Failed,
    FailureRate,
    TimeSavedMinutes,
    AverageRunTimeMs,
}

impl SortKey {
    fn json_key(self) -> &'static str {
        match self {
            Self::WorkflowName => "workflowName",
            Self::Total => "total",
            Self::Failed => "failed",
            Self::FailureRate => "failureRate",
            Self::TimeSavedMinutes => "timeSavedMinutes",
            Self::AverageRunTimeMs => "averageRunTimeMs",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::WorkflowName => "Name",
            Self::Total => "Prod. executions",
            Self::Failed => "Failed prod. executions",
            Self::FailureRate => "Failure rate",
            Self::TimeSavedMinutes => "Time saved",
            Self::AverageRunTimeMs => "Run time (avg.)",
        }
    }
}

const METRICS: [MetricId; 5] = [
    MetricId::Total,
    MetricId::Failed,
    MetricId::FailureRate,
    MetricId::TimeSaved,
    MetricId::AverageRunTime,
];

const TABLE_COLUMNS: [SortKey; 6] = [
    SortKey::WorkflowName,
    SortKey::Total,
    SortKey::Failed,
    SortKey::FailureRate,
    SortKey::TimeSavedMinutes,
    SortKey::AverageRunTimeMs,
];

fn fmt_count(n: f64) -> String {
    format!("{}", (n.round() as i64).separate_with_commas_en())
}

trait SeparateCommas {
    fn separate_with_commas_en(self) -> String;
}

impl SeparateCommas for i64 {
    fn separate_with_commas_en(self) -> String {
        let s = self.abs().to_string();
        let mut out = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let formatted: String = out.chars().rev().collect();
        if self < 0 {
            format!("-{formatted}")
        } else {
            formatted
        }
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

fn fmt_time_saved(minutes: f64) -> String {
    if minutes >= 60.0 {
        let hours = minutes / 60.0;
        if hours >= 100.0 {
            format!("{}h", (hours.round() as i64).separate_with_commas_en())
        } else if hours >= 10.0 {
            format!("{:.0}h", hours)
        } else {
            format!("{:.1}h", hours)
        }
    } else {
        format!("{}m", minutes.round() as i64)
    }
}

fn range_label(days: i32) -> String {
    format!("Last {days} days")
}

fn format_range(from: &str, to: &str) -> String {
    let fmt = |s: &str| -> String {
        chrono::NaiveDate::parse_from_str(&s[..s.len().min(10)], "%Y-%m-%d")
            .map(|d| d.format("%b %d %Y").to_string())
            .unwrap_or_else(|_| s.to_string())
    };
    format!("{} – {}", fmt(from), fmt(to))
}

fn metric_number(insights: &serde_json::Value, key: &str) -> (f64, Option<f64>) {
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

fn metric_display(id: MetricId, insights: &serde_json::Value) -> (String, f64, Option<f64>) {
    let (raw, previous) = metric_number(insights, id.key());
    let display = match id {
        MetricId::Total | MetricId::Failed => fmt_count(raw),
        MetricId::FailureRate => fmt_ratio(raw),
        MetricId::TimeSaved => fmt_time_saved(raw),
        MetricId::AverageRunTime => fmt_ms(raw),
    };
    (display, raw, previous)
}

#[derive(Clone)]
struct Trend {
    text: String,
    kind: TrendKind,
    dir: TrendDir,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrendKind {
    Positive,
    Negative,
    Neutral,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrendDir {
    Up,
    Down,
    Flat,
}

fn impact_of(id: MetricId) -> &'static str {
    match id {
        MetricId::Failed | MetricId::FailureRate | MetricId::AverageRunTime => "lower",
        MetricId::TimeSaved | MetricId::Total => "higher",
    }
}

fn trend_for(id: MetricId, current: f64, previous: Option<f64>) -> Option<Trend> {
    let previous = previous?;
    let d = current - previous;
    if d.abs() < 1e-9 {
        return Some(Trend {
            text: "±0".into(),
            kind: TrendKind::Neutral,
            dir: TrendDir::Flat,
        });
    }
    let dir = if d > 0.0 {
        TrendDir::Up
    } else {
        TrendDir::Down
    };
    let text = match id {
        MetricId::FailureRate => format!("{:+.1}pp", d * 100.0),
        MetricId::TimeSaved => {
            let sign = if d > 0.0 { "+" } else { "−" };
            format!("{sign}{}", fmt_time_saved(d.abs()))
        }
        MetricId::AverageRunTime => {
            let sign = if d > 0.0 { "+" } else { "−" };
            format!("{sign}{}", fmt_ms(d.abs()))
        }
        MetricId::Total | MetricId::Failed => {
            let pct = if previous != 0.0 {
                (d / previous) * 100.0
            } else {
                0.0
            };
            format!("{:.1}%", pct.abs())
        }
    };
    let kind = match impact_of(id) {
        "higher" => {
            if d > 0.0 {
                TrendKind::Positive
            } else {
                TrendKind::Negative
            }
        }
        "lower" => {
            if d < 0.0 {
                TrendKind::Positive
            } else {
                TrendKind::Negative
            }
        }
        _ => TrendKind::Neutral,
    };
    Some(Trend { text, kind, dir })
}

fn series_value(id: MetricId, point: &serde_json::Value) -> f64 {
    match id {
        MetricId::Total => point.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0),
        MetricId::Failed => point.get("failed").and_then(|v| v.as_f64()).unwrap_or(0.0),
        MetricId::FailureRate => point.get("failureRate").and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0,
        MetricId::TimeSaved => point.get("timeSavedMinutes").and_then(|v| v.as_f64()).unwrap_or(0.0) / 60.0,
        MetricId::AverageRunTime => {
            point.get("averageRunTimeMs").and_then(|v| v.as_f64()).unwrap_or(0.0) / 1000.0
        }
    }
}

fn format_tick(metric: MetricId, v: f64) -> String {
    match metric {
        MetricId::FailureRate => format!("{:.1}", v),
        MetricId::AverageRunTime => format!("{:.2}", v),
        _ => fmt_count(v),
    }
}

#[component]
fn AreaChart(data: Vec<serde_json::Value>, metric: MetricId) -> Element {
    let width = 720.0;
    let height = 220.0;
    let pad_top = 16.0;
    let pad_right = 12.0;
    let pad_bottom = 28.0;
    let pad_left = 44.0;
    let values: Vec<f64> = data.iter().map(|d| series_value(metric, d)).collect();
    let max = values.iter().cloned().fold(0.0, f64::max);
    let nice_max = if max == 0.0 { 1.0 } else { max * 1.1 };
    let inner_w = width - pad_left - pad_right;
    let inner_h = height - pad_top - pad_bottom;

    let points: Vec<(f64, f64, String)> = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let x = pad_left
                + if data.len() <= 1 {
                    inner_w / 2.0
                } else {
                    (i as f64 / (data.len() - 1) as f64) * inner_w
                };
            let y = pad_top + inner_h - (series_value(metric, d) / nice_max) * inner_h;
            let label = d.get("date").map(json_string).unwrap_or_default();
            (x, y, label)
        })
        .collect();

    let line = if points.is_empty() {
        String::new()
    } else {
        points
            .iter()
            .enumerate()
            .map(|(i, (x, y, _))| {
                if i == 0 {
                    format!("M{x:.1},{y:.1}")
                } else {
                    format!("L{x:.1},{y:.1}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let area = if points.is_empty() {
        String::new()
    } else {
        let bottom = pad_top + inner_h;
        format!(
            "{line} L{:.1},{bottom:.1} L{:.1},{bottom:.1} Z",
            points.last().map(|p| p.0).unwrap_or(0.0),
            points.first().map(|p| p.0).unwrap_or(0.0)
        )
    };

    let ticks: [f64; 3] = [0.0, 0.5 * nice_max, nice_max];
    let label_stride = if points.len() <= 6 {
        1
    } else {
        (points.len() as f64 / 6.0).ceil() as usize
    };
    let mid_label = metric.chart_unit();

    rsx! {
        svg {
            view_box: "0 0 {width} {height}",
            class: "h-[220px] w-full",
            role: "img",
            defs {
                linearGradient {
                    id: "insightsFill",
                    x1: "0",
                    y1: "0",
                    x2: "0",
                    y2: "1",
                    stop { offset: "0%", stop_color: "var(--primary)", stop_opacity: "0.35" }
                    stop { offset: "100%", stop_color: "var(--primary)", stop_opacity: "0.02" }
                }
            }
            for (i, v) in ticks.iter().enumerate() {
                {
                    let y = pad_top + inner_h - (v / nice_max) * inner_h;
                    let tick = format_tick(metric, *v);
                    rsx! {
                        g {
                            key: "tick-{i}",
                            line {
                                x1: pad_left,
                                x2: width - pad_right,
                                y1: y,
                                y2: y,
                                stroke: "var(--border)",
                                stroke_dasharray: "3 4",
                            }
                            text {
                                x: pad_left - 8.0,
                                y: y + 3.0,
                                text_anchor: "end",
                                class: "fill-muted-foreground",
                                font_size: "10",
                                "{tick}"
                            }
                        }
                    }
                }
            }
            text {
                x: 12.0,
                y: pad_top + inner_h / 2.0,
                transform: "rotate(-90 12 {pad_top + inner_h / 2.0})",
                class: "fill-muted-foreground",
                font_size: "10",
                text_anchor: "middle",
                "{mid_label}"
            }
            if !area.is_empty() {
                path { d: "{area}", fill: "url(#insightsFill)" }
            }
            if !line.is_empty() {
                path {
                    d: "{line}",
                    fill: "none",
                    stroke: "var(--primary)",
                    stroke_width: "2",
                    stroke_linejoin: "round",
                    stroke_linecap: "round",
                }
            }
            for (i, (x, _y, label)) in points.iter().enumerate() {
                {
                    let show = i == 0
                        || i == points.len().saturating_sub(1)
                        || (label_stride > 0 && i % label_stride == 0);
                    let date_label = if show {
                        chrono::NaiveDate::parse_from_str(&label[..label.len().min(10)], "%Y-%m-%d")
                            .map(|d| d.format("%b %d").to_string())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    rsx! {
                        text {
                            key: "{label}-{i}",
                            x: *x,
                            y: height - 8.0,
                            text_anchor: "middle",
                            class: "fill-muted-foreground",
                            font_size: "10",
                            "{date_label}"
                        }
                    }
                }
            }
        }
    }
}

fn row_sort_value(row: &serde_json::Value, key: SortKey) -> serde_json::Value {
    row.get(key.json_key())
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn format_cell(key: SortKey, row: &serde_json::Value) -> String {
    match key {
        SortKey::WorkflowName => row.get("workflowName").map(json_string).unwrap_or_default(),
        SortKey::Total | SortKey::Failed => {
            fmt_count(row.get(key.json_key()).and_then(|v| v.as_f64()).unwrap_or(0.0))
        }
        SortKey::FailureRate => fmt_ratio(row.get("failureRate").and_then(|v| v.as_f64()).unwrap_or(0.0)),
        SortKey::TimeSavedMinutes => {
            fmt_time_saved(row.get("timeSavedMinutes").and_then(|v| v.as_f64()).unwrap_or(0.0))
        }
        SortKey::AverageRunTimeMs => {
            fmt_ms(row.get("averageRunTimeMs").and_then(|v| v.as_f64()).unwrap_or(0.0))
        }
    }
}

#[component]
pub fn InsightsPanel(#[props(default)] refresh_key: u32) -> Element {
    let mut insights = use_signal(|| None::<serde_json::Value>);
    let mut days = use_signal(|| 7_i32);
    let mut err = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    let mut tick = use_signal(|| 0_u32);
    let mut selected = use_signal(|| MetricId::TimeSaved);
    let mut sort_key = use_signal(|| SortKey::TimeSavedMinutes);
    let mut sort_desc = use_signal(|| true);

    use_effect(move || {
        let _ = refresh_key;
        let _ = tick();
        let d = days();
        loading.set(true);
        spawn(async move {
            let path = format!("/api/insights?days={d}");
            match crate::api::get_json(&path).await {
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
        });
    });

    let mut rows = insights()
        .as_ref()
        .and_then(|i| i.get("byWorkflow"))
        .and_then(|w| w.as_array())
        .cloned()
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        let av = row_sort_value(a, sort_key());
        let bv = row_sort_value(b, sort_key());
        let cmp = match (&av, &bv) {
            (serde_json::Value::String(a), serde_json::Value::String(b)) => a.cmp(b),
            _ => av
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&bv.as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        if sort_desc() {
            cmp.reverse()
        } else {
            cmp
        }
    });

    let by_time = insights()
        .as_ref()
        .and_then(|i| i.get("byTime"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let range_chip = insights().as_ref().map(|i| {
        let from = i.get("from").map(json_string).unwrap_or_default();
        let to = i.get("to").map(json_string).unwrap_or_default();
        format_range(&from, &to)
    });

    let selected_meta = selected();

    rsx! {
        div { class: "flex h-full flex-col gap-3 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                h2 { class: "text-base font-semibold tracking-tight", "Insights" }
                div { class: "flex-1" }
                if let Some(chip) = range_chip {
                    span { class: "rounded-md border bg-card px-2.5 py-1 text-xs text-muted-foreground", "{chip}" }
                }
                div { class: "flex overflow-hidden rounded-md border",
                    for d in [7_i32, 14, 30] {
                        button {
                            key: "{d}",
                            class: if days() == d {
                                "bg-primary px-2.5 py-1 text-xs text-primary-foreground"
                            } else {
                                "bg-card px-2.5 py-1 text-xs text-muted-foreground hover:bg-muted"
                            },
                            onclick: move |_| days.set(d),
                            "{d}d"
                        }
                    }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    disabled: loading(),
                    onclick: move |_| tick.set(tick() + 1),
                    RefreshCw { class: Some(format!("h-3.5 w-3.5{}", if loading() { " animate-spin" } else { "" })) }
                    "Refresh"
                }
            }

            if let Some(message) = err() {
                div { class: "rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive",
                    "{message}"
                }
            }

            div { class: "grid grid-cols-2 gap-2 lg:grid-cols-5",
                for m in METRICS {
                    {
                        let active = selected() == m;
                        let (display, raw, prev) = insights()
                            .as_ref()
                            .map(|i| metric_display(m, i))
                            .unwrap_or_else(|| {
                                if loading() {
                                    ("…".into(), 0.0, None)
                                } else {
                                    ("—".into(), 0.0, None)
                                }
                            });
                        let trend = insights()
                            .as_ref()
                            .and_then(|_| trend_for(m, raw, prev));
                        let trend_class = trend.as_ref().map(|t| match t.kind {
                            TrendKind::Positive => "text-primary",
                            TrendKind::Negative => "text-destructive",
                            TrendKind::Neutral => "text-muted-foreground",
                        }).unwrap_or("");
                        rsx! {
                            button {
                                key: "{m.key()}",
                                class: if active {
                                    "rounded-md border border-primary px-3 py-2.5 text-left ring-1 ring-primary/40 transition-colors hover:bg-accent/40"
                                } else {
                                    "rounded-md border bg-card px-3 py-2.5 text-left transition-colors hover:bg-accent/40"
                                },
                                onclick: move |_| {
                                    selected.set(m);
                                    sort_key.set(m.sort_key());
                                    sort_desc.set(true);
                                },
                                div { class: "text-xs font-medium text-foreground", "{m.label()}" }
                                div { class: "text-[10px] text-muted-foreground", "{range_label(days())}" }
                                div { class: "mt-2 flex items-end justify-between gap-2",
                                    div { class: "text-xl font-semibold tabular-nums tracking-tight", "{display}" }
                                    if let Some(t) = trend {
                                        span {
                                            class: "inline-flex items-center gap-0.5 text-[11px] tabular-nums {trend_class}",
                                            if t.dir == TrendDir::Up {
                                                ArrowUp { class: Some("h-3 w-3".into()) }
                                            } else if t.dir == TrendDir::Down {
                                                ArrowDown { class: Some("h-3 w-3".into()) }
                                            } else {
                                                ChevronRight { class: Some("h-3 w-3".into()) }
                                            }
                                            "{t.text}"
                                        }
                                    }
                                }
                                div {
                                    class: if active { "mt-2 h-0.5 rounded-full bg-primary" } else { "mt-2 h-0.5 rounded-full bg-transparent" },
                                }
                            }
                        }
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-hidden rounded-md border bg-card",
                div { class: "flex h-full flex-col gap-4 overflow-y-auto p-3",
                    div { class: "rounded-md border bg-background/40 p-2",
                        if by_time.is_empty() {
                            div { class: "flex h-[220px] items-center justify-center text-sm text-muted-foreground",
                                if loading() {
                                    "Loading chart…"
                                } else {
                                    "No execution data in this window"
                                }
                            }
                        } else {
                            AreaChart { data: by_time, metric: selected() }
                        }
                    }
                    div {
                        h3 { class: "mb-2 text-sm font-semibold", "Breakdown by workflow" }
                        div { class: "overflow-x-auto rounded-md border",
                            table { class: "w-full min-w-[720px] text-left text-sm",
                                thead { class: "border-b bg-muted/40 text-xs text-muted-foreground",
                                    tr {
                                        for col in TABLE_COLUMNS {
                                            {
                                                let active = sort_key() == col;
                                                rsx! {
                                                    th { class: "px-3 py-2 font-medium",
                                                        key: "{col.json_key()}",
                                                        button {
                                                            class: "inline-flex items-center gap-1 hover:text-foreground",
                                                            onclick: move |_| {
                                                                if sort_key() == col {
                                                                    sort_desc.set(!sort_desc());
                                                                } else {
                                                                    sort_key.set(col);
                                                                    sort_desc.set(true);
                                                                }
                                                            },
                                                            "{col.label()}"
                                                            if active {
                                                                if sort_desc() {
                                                                    ArrowDown { class: Some("h-3 w-3".into()) }
                                                                } else {
                                                                    ArrowUp { class: Some("h-3 w-3".into()) }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                tbody {
                                    for row in rows.iter() {
                                        {
                                            let wf_id = row.get("workflowId").map(json_string).unwrap_or_default();
                                            let name = row.get("workflowName").map(json_string).unwrap_or_default();
                                            rsx! {
                                                tr { class: "border-b last:border-0 hover:bg-muted/30",
                                                    key: "{wf_id}",
                                                    td { class: "px-3 py-2.5",
                                                        span { class: "font-medium underline-offset-2 hover:underline", "{name}" }
                                                    }
                                                    for col in [SortKey::Total, SortKey::Failed, SortKey::FailureRate, SortKey::TimeSavedMinutes, SortKey::AverageRunTimeMs] {
                                                        td { class: "px-3 py-2.5 tabular-nums", "{format_cell(col, row)}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !loading() && insights().is_some() && rows.is_empty() {
                                        tr {
                                            td {
                                                colspan: "6",
                                                class: "px-3 py-8 text-center text-muted-foreground",
                                                "No executions in this window. Run a workflow to see metrics for {selected_meta.label().to_lowercase()}."
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