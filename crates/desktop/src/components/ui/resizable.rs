use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResizableOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct PanelGroupCtx {
    orientation: ResizableOrientation,
    group_id: String,
    sizes: Signal<Vec<f64>>,
    mins: Signal<Vec<f64>>,
    maxs: Signal<Vec<f64>>,
    drag: Signal<Option<DragState>>,
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    handle: usize,
    start_pos: f64,
    start_sizes: [f64; 2],
    container_px: f64,
}

fn flex_dir(orientation: ResizableOrientation) -> &'static str {
    match orientation {
        ResizableOrientation::Horizontal => "flex-row",
        ResizableOrientation::Vertical => "flex-col",
    }
}

fn cursor(orientation: ResizableOrientation) -> &'static str {
    match orientation {
        ResizableOrientation::Horizontal => "cursor-col-resize",
        ResizableOrientation::Vertical => "cursor-row-resize",
    }
}

fn clamp_pair(
    a: f64,
    b: f64,
    min_a: f64,
    max_a: f64,
    min_b: f64,
    max_b: f64,
) -> (f64, f64) {
    let mut a = a;
    let mut b = b;
    if a < min_a {
        b -= min_a - a;
        a = min_a;
    }
    if b < min_b {
        a -= min_b - b;
        b = min_b;
    }
    if a > max_a {
        b += a - max_a;
        a = max_a;
    }
    if b > max_b {
        a += b - max_b;
        b = max_b;
    }
    (a.clamp(min_a, max_a), b.clamp(min_b, max_b))
}

async fn measure_group_px(group_id: &str, orientation: ResizableOrientation) -> f64 {
    let axis = match orientation {
        ResizableOrientation::Horizontal => "width",
        ResizableOrientation::Vertical => "height",
    };
    let script = format!(
        r#"(function(){{
  const el = document.getElementById({id});
  if (!el) return 0;
  const r = el.getBoundingClientRect();
  return r.{axis} || 0;
}})()"#,
        id = serde_json::to_string(group_id).unwrap_or_else(|_| "\"\"".into()),
        axis = axis,
    );
    document::eval(&script)
        .await
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

#[component]
pub fn ResizablePanelGroup(
    orientation: ResizableOrientation,
    #[props(default = 2)] panel_count: usize,
    #[props(default)] initial_sizes: Vec<f64>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let count = panel_count.max(1);
    let default_sizes = if initial_sizes.len() == count {
        let sum: f64 = initial_sizes.iter().sum();
        if sum > 0.0 {
            initial_sizes
                .iter()
                .map(|s| s / sum * 100.0)
                .collect::<Vec<_>>()
        } else {
            vec![100.0 / count as f64; count]
        }
    } else {
        vec![100.0 / count as f64; count]
    };
    let sizes = use_signal(|| default_sizes);
    let mins = use_signal(|| vec![10.0; count]);
    let maxs = use_signal(|| vec![90.0; count]);
    let drag = use_signal(|| None::<DragState>);
    let group_id = use_hook(|| format!("bf-resize-{}", Uuid::new_v4()));

    let ctx = PanelGroupCtx {
        orientation,
        group_id: group_id.clone(),
        sizes,
        mins,
        maxs,
        drag,
    };
    use_context_provider(|| ctx.clone());

    let base = class.unwrap_or_else(|| "min-h-0 min-w-0 flex-1".into());
    let cls = format!(
        "relative flex h-full min-h-0 min-w-0 w-full {base} {}",
        flex_dir(orientation)
    );
    let dragging = (ctx.drag)().is_some();

    rsx! {
        div {
            id: "{group_id}",
            class: "{cls}",
            "data-bf-resizable": "1",
            // Capture moves on the group while dragging so WebView doesn't lose the gesture.
            onmousemove: {
                let mut ctx = ctx.clone();
                move |e| {
                    let Some(st) = (ctx.drag)() else { return };
                    let pos = if ctx.orientation == ResizableOrientation::Horizontal {
                        e.client_coordinates().x
                    } else {
                        e.client_coordinates().y
                    };
                    let delta_pct = if st.container_px > 1.0 {
                        (pos - st.start_pos) / st.container_px * 100.0
                    } else {
                        0.0
                    };
                    let i = st.handle;
                    let mut a = st.start_sizes[0] + delta_pct;
                    let mut b = st.start_sizes[1] - delta_pct;
                    let (min_a, max_a, min_b, max_b) = ctx.mins.with(|m| {
                        ctx.maxs.with(|x| {
                            (
                                *m.get(i).unwrap_or(&10.0),
                                *x.get(i).unwrap_or(&90.0),
                                *m.get(i + 1).unwrap_or(&10.0),
                                *x.get(i + 1).unwrap_or(&90.0),
                            )
                        })
                    });
                    (a, b) = clamp_pair(a, b, min_a, max_a, min_b, max_b);
                    ctx.sizes.with_mut(|sizes| {
                        if i + 1 < sizes.len() {
                            sizes[i] = a;
                            sizes[i + 1] = b;
                        }
                    });
                }
            },
            onmouseup: {
                let mut ctx = ctx.clone();
                move |_| {
                    if (ctx.drag)().is_some() {
                        ctx.drag.set(None);
                    }
                }
            },
            onmouseleave: {
                let mut ctx = ctx.clone();
                move |_| {
                    // Keep dragging if pointer left briefly; fixed overlay below covers the rest.
                    let _ = &ctx;
                }
            },
            {children}
            if dragging {
                div {
                    class: "fixed inset-0 z-[400] {cursor(orientation)}",
                    style: "touch-action:none;",
                    onmousemove: {
                        let mut ctx = ctx.clone();
                        move |e| {
                            let Some(st) = (ctx.drag)() else { return };
                            let pos = if ctx.orientation == ResizableOrientation::Horizontal {
                                e.client_coordinates().x
                            } else {
                                e.client_coordinates().y
                            };
                            let delta_pct = if st.container_px > 1.0 {
                                (pos - st.start_pos) / st.container_px * 100.0
                            } else {
                                0.0
                            };
                            let i = st.handle;
                            let mut a = st.start_sizes[0] + delta_pct;
                            let mut b = st.start_sizes[1] - delta_pct;
                            let (min_a, max_a, min_b, max_b) = ctx.mins.with(|m| {
                                ctx.maxs.with(|x| {
                                    (
                                        *m.get(i).unwrap_or(&10.0),
                                        *x.get(i).unwrap_or(&90.0),
                                        *m.get(i + 1).unwrap_or(&10.0),
                                        *x.get(i + 1).unwrap_or(&90.0),
                                    )
                                })
                            });
                            (a, b) = clamp_pair(a, b, min_a, max_a, min_b, max_b);
                            ctx.sizes.with_mut(|sizes| {
                                if i + 1 < sizes.len() {
                                    sizes[i] = a;
                                    sizes[i + 1] = b;
                                }
                            });
                        }
                    },
                    onmouseup: {
                        let mut ctx = ctx.clone();
                        move |_| ctx.drag.set(None)
                    },
                }
            }
        }
    }
}

#[component]
pub fn ResizablePanel(
    panel_index: usize,
    #[props(default = 10.0)] min_pct: f64,
    #[props(default = 90.0)] max_pct: f64,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let mut ctx = use_context::<PanelGroupCtx>();
    let idx = panel_index;

    use_hook(move || {
        ctx.mins.with_mut(|m| {
            if idx < m.len() {
                m[idx] = min_pct;
            }
        });
        ctx.maxs.with_mut(|x| {
            if idx < x.len() {
                x[idx] = max_pct;
            }
        });
    });

    let pct = (ctx.sizes)().get(idx).copied().unwrap_or(50.0);
    let extra = class.unwrap_or_else(|| "min-h-0 min-w-0".into());
    let style = if ctx.orientation == ResizableOrientation::Horizontal {
        format!("flex: 0 0 {pct:.4}%; width: {pct:.4}%; max-width: {pct:.4}%;")
    } else {
        format!("flex: 0 0 {pct:.4}%; height: {pct:.4}%; max-height: {pct:.4}%;")
    };

    rsx! {
        div {
            class: "min-h-0 min-w-0 overflow-hidden {extra}",
            style: "{style}",
            {children}
        }
    }
}

#[component]
pub fn ResizableHandle(
    #[props(default = 0)] handle_index: usize,
    #[props(default = true)] with_handle: bool,
    /// Ignored — size is measured from the panel group on drag start.
    #[props(default = 640.0)]
    container_px: f64,
) -> Element {
    let mut ctx = use_context::<PanelGroupCtx>();
    let idx = handle_index;
    let orient = ctx.orientation;
    let group_id = ctx.group_id.clone();
    let _ = container_px;

    let (track, grip) = match orient {
        ResizableOrientation::Horizontal => (
            "group relative z-20 flex w-2 shrink-0 cursor-col-resize items-center justify-center bg-border/60 hover:bg-[var(--brand)]/50 active:bg-[var(--brand)]/70",
            "h-8 w-1 rounded-full bg-muted-foreground/50 group-hover:bg-[var(--brand)]",
        ),
        ResizableOrientation::Vertical => (
            "group relative z-30 -my-0.5 flex h-3 shrink-0 cursor-row-resize items-center justify-center bg-border hover:bg-[var(--brand)]/50 active:bg-[var(--brand)]/70",
            "h-1 w-12 rounded-full bg-muted-foreground/70 group-hover:bg-[var(--brand)]",
        ),
    };

    rsx! {
        div {
            class: "{track}",
            title: "Drag to resize",
            role: "separator",
            "aria-orientation": if orient == ResizableOrientation::Horizontal { "vertical" } else { "horizontal" },
            onmousedown: move |e| {
                e.prevent_default();
                e.stop_propagation();
                let pos = if orient == ResizableOrientation::Horizontal {
                    e.client_coordinates().x
                } else {
                    e.client_coordinates().y
                };
                let (a, b) = ctx.sizes.with(|sizes| {
                    if idx + 1 < sizes.len() {
                        (sizes[idx], sizes[idx + 1])
                    } else {
                        (50.0, 50.0)
                    }
                });
                let group_id = group_id.clone();
                let mut ctx = ctx.clone();
                spawn(async move {
                    let measured = measure_group_px(&group_id, orient).await;
                    let px = if measured > 1.0 { measured } else { 960.0 };
                    ctx.drag.set(Some(DragState {
                        handle: idx,
                        start_pos: pos,
                        start_sizes: [a, b],
                        container_px: px,
                    }));
                });
            },
            if with_handle {
                div { class: "{grip}" }
            }
        }
    }
}
