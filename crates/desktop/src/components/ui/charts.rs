//! Rust/UI AreaChart — https://rust-ui.com/charts/area-chart
//! ApexCharts wrapper + SVG fallback (desktop webview often skips 2nd Apex instance).

use dioxus::prelude::*;
use tw_merge::tw_merge;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartCurve {
    #[default]
    Smooth,
    Straight,
    Stepline,
}

impl ChartCurve {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Straight => "Straight",
            Self::Stepline => "Stepline",
        }
    }
}

#[component]
pub fn AreaChart(
    json_values: String,
    json_labels: String,
    #[props(default)] curve: ChartCurve,
    #[props(default)] class: Option<String>,
    #[props(default)] series_names: Option<String>,
    #[props(default)] stack_type: Option<String>,
    #[props(default)] gradient: Option<bool>,
    #[props(default)] show_yaxis: Option<bool>,
    #[props(default)] show_grid: Option<bool>,
    #[props(default)] json_annotations: Option<String>,
    #[props(default)] value_prefix: Option<String>,
    #[props(default)] value_suffix: Option<String>,
) -> Element {
    let chart_id = use_hook(|| format!("bf-area-{}", Uuid::new_v4()));
    let merged_class = tw_merge!("w-full", class.as_deref().unwrap_or(""));
    let series_names_s = series_names.clone().unwrap_or_default();
    let stack_type_s = stack_type.clone().unwrap_or_default();
    let annotations_s = json_annotations.clone().unwrap_or_default();
    let value_prefix_s = value_prefix.clone().unwrap_or_default();
    let value_suffix_s = value_suffix.clone().unwrap_or_default();
    let gradient_s = gradient
        .map(|g| if g { "true" } else { "false" })
        .unwrap_or_default();
    let yaxis_s = show_yaxis
        .map(|y| if y { "true" } else { "false" })
        .unwrap_or_default();
    let grid_s = show_grid
        .map(|g| if g { "true" } else { "false" })
        .unwrap_or_default();

    let values_for_effect = json_values.clone();
    let labels_for_effect = json_labels.clone();
    let id_for_effect = chart_id.clone();
    use_effect(move || {
        let _ = values_for_effect.len();
        let _ = labels_for_effect.len();
        let id = id_for_effect.clone();
        spawn(async move {
            crate::chart_assets::ensure_charts_loaded().await;
            // Second chart (e.g. Latency) often mounts after the first init pass —
            // clear its flag and refresh once layout has a real height.
            let _ = document::eval(&format!(
                r#"
                (function(){{
                  const el = document.getElementById("{id}");
                  if (el) delete el.dataset.chartInitialized;
                  const kick = () => {{
                    if (window.__bfRefreshCharts) window.__bfRefreshCharts();
                  }};
                  kick();
                  requestAnimationFrame(() => setTimeout(kick, 80));
                  return true;
                }})()
                "#
            ))
            .await;
        });
    });

    rsx! {
        div {
            id: "{chart_id}",
            class: "{merged_class} min-h-[200px]",
            "data-name": "AreaChart",
            "data-chart-curve": curve.as_str(),
            "data-chart-values": "{json_values}",
            "data-chart-labels": "{json_labels}",
            "data-chart-series-names": "{series_names_s}",
            "data-chart-stack-type": "{stack_type_s}",
            "data-chart-gradient": "{gradient_s}",
            "data-chart-show-yaxis": "{yaxis_s}",
            "data-chart-show-grid": "{grid_s}",
            "data-chart-annotations": "{annotations_s}",
            "data-chart-value-prefix": "{value_prefix_s}",
            "data-chart-value-suffix": "{value_suffix_s}",
        }
    }
}

fn short_axis_label(raw: &str) -> String {
    // Hourly: "2024-11-27 13:00" → "13:00"
    if let Some(caps) = raw.split_whitespace().nth(1) {
        if caps.len() >= 5 {
            return caps[..5].to_string();
        }
    }
    // Daily: "2024-11-27" → "Nov 27"
    if raw.len() >= 10 {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&raw[..10], "%Y-%m-%d") {
            return d.format("%b %d").to_string();
        }
    }
    raw.to_string()
}

/// Reliable SVG area chart (same look as Traffic). Use when ApexCharts won't paint.
#[component]
pub fn SvgAreaChart(
    labels: Vec<String>,
    values: Vec<f64>,
    #[props(default = "Duration".to_string())] series_name: String,
    #[props(default)] value_suffix: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let width = 720.0_f64;
    let height = 220.0_f64;
    let pad_top = 16.0;
    let pad_right = 12.0;
    let pad_bottom = 36.0;
    let pad_left = 48.0;
    let inner_w = width - pad_left - pad_right;
    let inner_h = height - pad_top - pad_bottom;
    let n = values.len().max(1);
    let max_v = values.iter().cloned().fold(0.0_f64, f64::max);
    let nice_max = if max_v <= 0.0 { 1.0 } else { (max_v * 1.15).ceil().max(1.0) };
    let suffix = value_suffix.unwrap_or_default();
    let grad_id = use_hook(|| format!("bf-svg-fill-{}", Uuid::new_v4()));

    let points: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = pad_left
                + if n <= 1 {
                    inner_w / 2.0
                } else {
                    (i as f64 / (n - 1) as f64) * inner_w
                };
            let y = pad_top + inner_h - (v / nice_max) * inner_h;
            (x, y)
        })
        .collect();

    let line = points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if i == 0 {
                format!("M{x:.1},{y:.1}")
            } else {
                format!("L{x:.1},{y:.1}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
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

    let ticks: [f64; 4] = [0.0, nice_max / 3.0, (nice_max * 2.0) / 3.0, nice_max];
    let label_stride = if labels.len() <= 10 {
        1
    } else {
        (labels.len() as f64 / 8.0).ceil() as usize
    };
    let merged = tw_merge!("w-full", class.as_deref().unwrap_or(""));

    rsx! {
        div { class: "{merged} flex flex-col",
            svg {
                view_box: "0 0 {width} {height}",
                class: "h-[220px] w-full",
                role: "img",
                "aria-label": "{series_name}",
                defs {
                    linearGradient {
                        id: "{grad_id}",
                        x1: "0",
                        y1: "0",
                        x2: "0",
                        y2: "1",
                        stop {
                            offset: "0%",
                            stop_color: "currentColor",
                            stop_opacity: "0.55",
                            class: "text-primary",
                        }
                        stop {
                            offset: "100%",
                            stop_color: "currentColor",
                            stop_opacity: "0.02",
                            class: "text-primary",
                        }
                    }
                }
                for (i, tick) in ticks.iter().enumerate() {
                    {
                        let y = pad_top + inner_h - (tick / nice_max) * inner_h;
                        let label = if suffix.is_empty() {
                            format!("{:.0}", tick)
                        } else {
                            format!("{:.0}{suffix}", tick)
                        };
                        rsx! {
                            g {
                                key: "tick-{i}",
                                line {
                                    x1: "{pad_left}",
                                    x2: "{width - pad_right}",
                                    y1: "{y}",
                                    y2: "{y}",
                                    stroke: "currentColor",
                                    class: "text-border",
                                    stroke_dasharray: "3 4",
                                    opacity: "0.7",
                                }
                                text {
                                    x: "{pad_left - 8.0}",
                                    y: "{y + 3.0}",
                                    text_anchor: "end",
                                    class: "fill-muted-foreground",
                                    font_size: "10",
                                    "{label}"
                                }
                            }
                        }
                    }
                }
                if !area.is_empty() {
                    path {
                        d: "{area}",
                        fill: "url(#{grad_id})",
                        class: "text-primary",
                    }
                }
                if !line.is_empty() {
                    path {
                        d: "{line}",
                        fill: "none",
                        stroke: "currentColor",
                        class: "text-primary",
                        stroke_width: "2",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                    }
                }
                for (i, ((x, y), label)) in points.iter().zip(labels.iter()).enumerate() {
                    {
                        let show = i == 0
                            || i == points.len().saturating_sub(1)
                            || (label_stride > 0 && i % label_stride == 0);
                        let axis = if show {
                            short_axis_label(label)
                        } else {
                            String::new()
                        };
                        let tip = if suffix.is_empty() {
                            format!("{label}: {:.0}", values.get(i).copied().unwrap_or(0.0))
                        } else {
                            format!("{label}: {:.0}{suffix}", values.get(i).copied().unwrap_or(0.0))
                        };
                        rsx! {
                            g {
                                key: "pt-{i}",
                                circle {
                                    cx: "{x}",
                                    cy: "{y}",
                                    r: "3",
                                    fill: "currentColor",
                                    class: "text-primary",
                                }
                                title { "{tip}" }
                                text {
                                    x: "{x}",
                                    y: "{height - 10.0}",
                                    text_anchor: "middle",
                                    class: "fill-muted-foreground",
                                    font_size: "10",
                                    "{axis}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "mt-1 flex items-center justify-center gap-2 text-[11px] text-muted-foreground",
                span { class: "inline-flex items-center gap-1.5",
                    span { class: "h-2 w-2 rounded-full bg-primary" }
                    "{series_name}"
                }
            }
        }
    }
}
