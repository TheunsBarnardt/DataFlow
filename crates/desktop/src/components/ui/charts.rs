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
    let merged_class = tw_merge!("w-full min-h-[200px]", class.as_deref().unwrap_or(""));
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
            // Second chart often mounts after the first init pass —
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
                  setTimeout(kick, 250);
                  setTimeout(kick, 600);
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
            class: "{merged_class}",
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

/// Catmull-Rom → cubic Bézier (Traffic-style smooth curve).
fn smooth_line_path(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        return format!("M{:.1},{:.1}", points[0].0, points[0].1);
    }
    let mut d = format!("M{:.1},{:.1}", points[0].0, points[0].1);
    for i in 0..points.len() - 1 {
        let p0 = if i == 0 { points[0] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() {
            points[i + 2]
        } else {
            points[i + 1]
        };
        let cp1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let cp1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let cp2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let cp2y = p2.1 - (p3.1 - p1.1) / 6.0;
        d.push_str(&format!(
            " C{cp1x:.1},{cp1y:.1} {cp2x:.1},{cp2y:.1} {:.1},{:.1}",
            p2.0, p2.1
        ));
    }
    d
}

fn area_under(line: &str, points: &[(f64, f64)], bottom: f64) -> String {
    if line.is_empty() || points.is_empty() {
        return String::new();
    }
    format!(
        "{line} L{:.1},{bottom:.1} L{:.1},{bottom:.1} Z",
        points.last().map(|p| p.0).unwrap_or(0.0),
        points.first().map(|p| p.0).unwrap_or(0.0)
    )
}

/// Band between a lower and upper smooth curve (for stacked “Slow” excess).
fn area_between(upper: &[(f64, f64)], lower: &[(f64, f64)]) -> String {
    if upper.is_empty() || lower.is_empty() || upper.len() != lower.len() {
        return String::new();
    }
    let up = smooth_line_path(upper);
    let mut down = String::new();
    for (i, p) in lower.iter().rev().enumerate() {
        if i == 0 {
            down.push_str(&format!("L{:.1},{:.1}", p.0, p.1));
        } else {
            // Reverse lower with matching smooth controls is hard; straight back is fine for fill.
            down.push_str(&format!(" L{:.1},{:.1}", p.0, p.1));
        }
    }
    format!("{up} {down} Z")
}

/// Reliable SVG area chart (same look/feel as Traffic). Use when ApexCharts won't paint.
/// When `slow_threshold` is set, values above it stack red (“Slow”) on green (“Normal”),
/// matching Traffic’s Success/Errors treatment for latency vs avg.
#[component]
pub fn SvgAreaChart(
    labels: Vec<String>,
    values: Vec<f64>,
    #[props(default = "Duration".to_string())] series_name: String,
    #[props(default)] value_suffix: Option<String>,
    #[props(default)] class: Option<String>,
    /// Avg latency (ms). Buckets above this render as red “Slow” excess.
    #[props(default)] slow_threshold: Option<f64>,
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
    let nice_max = if max_v <= 0.0 {
        1.0
    } else {
        (max_v * 1.15).ceil().max(1.0)
    };
    let suffix = value_suffix.unwrap_or_default();
    let threshold = slow_threshold.filter(|t| *t > 0.0);
    let has_slow = threshold
        .map(|t| values.iter().any(|&v| v > t))
        .unwrap_or(false);
    let uid = use_hook(|| format!("{}", Uuid::new_v4()));
    let grad_ok = format!("bf-svg-ok-{uid}");
    let grad_slow = format!("bf-svg-slow-{uid}");

    let x_at = |i: usize| -> f64 {
        pad_left
            + if n <= 1 {
                inner_w / 2.0
            } else {
                (i as f64 / (n - 1) as f64) * inner_w
            }
    };
    let y_at = |v: f64| -> f64 { pad_top + inner_h - (v / nice_max) * inner_h };

    let normal_vals: Vec<f64> = values
        .iter()
        .map(|&v| match threshold {
            Some(t) => v.min(t),
            None => v,
        })
        .collect();
    let total_points: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| (x_at(i), y_at(v)))
        .collect();
    let normal_points: Vec<(f64, f64)> = normal_vals
        .iter()
        .enumerate()
        .map(|(i, &v)| (x_at(i), y_at(v)))
        .collect();

    let bottom = pad_top + inner_h;
    let line = smooth_line_path(&total_points);
    let normal_line = smooth_line_path(&normal_points);
    let normal_area = area_under(&normal_line, &normal_points, bottom);
    let slow_area = if has_slow {
        area_between(&total_points, &normal_points)
    } else {
        String::new()
    };

    let ticks: [f64; 4] = [0.0, nice_max / 3.0, (nice_max * 2.0) / 3.0, nice_max];
    let label_stride = if labels.len() <= 10 {
        1
    } else {
        (labels.len() as f64 / 8.0).ceil() as usize
    };
    let merged = tw_merge!("w-full", class.as_deref().unwrap_or(""));
    let aria = if has_slow {
        format!("{series_name} (red when slower than avg)")
    } else {
        series_name.clone()
    };
    let legend_ok = if has_slow {
        "Normal".to_string()
    } else {
        series_name.clone()
    };
    let slow_segments: Vec<(usize, String)> = match threshold {
        Some(t) => (0..values.len().saturating_sub(1))
            .filter(|&i| values[i] > t || values[i + 1] > t)
            .map(|i| {
                (
                    i,
                    smooth_line_path(&[total_points[i], total_points[i + 1]]),
                )
            })
            .collect(),
        None => Vec::new(),
    };

    rsx! {
        div { class: "{merged} flex flex-col",
            svg {
                view_box: "0 0 {width} {height}",
                class: "h-[220px] w-full",
                role: "img",
                "aria-label": "{aria}",
                defs {
                    linearGradient {
                        id: "{grad_ok}",
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
                    linearGradient {
                        id: "{grad_slow}",
                        x1: "0",
                        y1: "0",
                        x2: "0",
                        y2: "1",
                        stop {
                            offset: "0%",
                            stop_color: "currentColor",
                            stop_opacity: "0.55",
                            class: "text-destructive",
                        }
                        stop {
                            offset: "100%",
                            stop_color: "currentColor",
                            stop_opacity: "0.08",
                            class: "text-destructive",
                        }
                    }
                }
                for (i, tick) in ticks.iter().enumerate() {
                    {
                        let y = y_at(*tick);
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
                if !normal_area.is_empty() {
                    path {
                        d: "{normal_area}",
                        fill: "url(#{grad_ok})",
                        class: "text-primary bf-svg-area",
                    }
                }
                if !slow_area.is_empty() {
                    path {
                        d: "{slow_area}",
                        fill: "url(#{grad_slow})",
                        class: "text-destructive bf-svg-area",
                    }
                }
                if !line.is_empty() {
                    // Stroke follows total height; color shifts via overlay segments below.
                    path {
                        d: "{line}",
                        fill: "none",
                        stroke: "currentColor",
                        class: "text-primary bf-svg-line",
                        stroke_width: "2",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                        "pathLength": "1",
                    }
                }
                // Red stroke on segments that cross / sit above avg latency.
                for (i, seg) in slow_segments.iter() {
                    path {
                        key: "slow-seg-{i}",
                        d: "{seg}",
                        fill: "none",
                        stroke: "currentColor",
                        class: "text-destructive bf-svg-line",
                        stroke_width: "2.25",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                        "pathLength": "1",
                    }
                }
                for (i, ((x, y), label)) in total_points.iter().zip(labels.iter()).enumerate() {
                    {
                        let show = i == 0
                            || i == total_points.len().saturating_sub(1)
                            || (label_stride > 0 && i % label_stride == 0);
                        let axis = if show {
                            short_axis_label(label)
                        } else {
                            String::new()
                        };
                        let v = values.get(i).copied().unwrap_or(0.0);
                        let slow_pt = threshold.map(|t| v > t).unwrap_or(false);
                        let tip = if suffix.is_empty() {
                            format!("{label}: {:.0}", v)
                        } else {
                            format!("{label}: {:.0}{suffix}", v)
                        };
                        let tip = if slow_pt {
                            format!("{tip} (slower than avg)")
                        } else {
                            tip
                        };
                        let dot_class = if slow_pt {
                            "text-destructive bf-svg-dot"
                        } else {
                            "text-primary bf-svg-dot"
                        };
                        rsx! {
                            g {
                                key: "pt-{i}",
                                circle {
                                    cx: "{x}",
                                    cy: "{y}",
                                    r: "3",
                                    fill: "currentColor",
                                    class: "{dot_class}",
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
            div { class: "mt-1 flex items-center justify-center gap-3 text-[11px] text-muted-foreground",
                span { class: "inline-flex items-center gap-1.5",
                    span { class: "h-2 w-2 rounded-full bg-primary" }
                    "{legend_ok}"
                }
                if has_slow {
                    span { class: "inline-flex items-center gap-1.5",
                        span { class: "h-2 w-2 rounded-full bg-destructive" }
                        "Slow"
                    }
                }
            }
        }
    }
}
