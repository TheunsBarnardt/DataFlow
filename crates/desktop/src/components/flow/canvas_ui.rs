//! Interactive canvas UI — menus, shortcuts, custom nodes/edges (React FlowCanvas parity).

use dioxus::prelude::*;
use dioxus_flow::prelude::{
    Background, BackgroundVariant, ConnectEnd, Controls, Edge, EdgeViewCtx, Flow, Handle,
    HandleKind, MiniMap, Node, NodeViewCtx, Point, Side,
};

use super::flow_canvas::{
    apply_run_overlay, studio_data_refresh, PendingAddRequest, StudioNodeData,
};
use super::handles::{
    is_trigger_type, node_accent, source_handle_label, source_handles_for_type, target_handle_label,
    target_handles_for_type,
};
use super::http_method_styles::{
    http_method_badge_class, is_http_method_label, is_item_count_label,
};
use super::node_glyphs::NodeGlyph;
use crate::components::icons::{Copy, Pencil, Play, Plus, Trash2, X};
use crate::components::ui::ConfirmDeleteDialog;

fn format_duration_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{}ms", ms.round() as i64)
    } else {
        format!("{:.1}s", ms / 1000.0)
    }
}

fn clamp_menu(x: f64, y: f64) -> (f64, f64) {
    let pad = 8.0;
    (x.max(pad), y.max(pad))
}

fn edge_label_chip(lbl: &str, style: &str) -> Element {
    if is_item_count_label(lbl) {
        let failed = style.contains("--destructive");
        let pill = if failed {
            "rounded-full border border-destructive/40 bg-destructive/10 px-1.5 py-0.5 text-[10px] font-medium text-destructive shadow-sm"
        } else {
            "rounded-full border border-emerald-500/40 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:text-emerald-300 shadow-sm"
        };
        return rsx! {
            span { class: "{pill}", "{lbl}" }
        };
    }
    if is_http_method_label(lbl) {
        let badge = http_method_badge_class(lbl);
        return rsx! {
            span {
                class: "rounded px-1 py-0.5 text-[10px] font-semibold shadow-sm {badge}",
                "{lbl}"
            }
        };
    }
    rsx! {
        span {
            class: "rounded bg-[var(--canvas)] px-1 py-0.5 text-[10px] font-medium text-muted-foreground shadow-sm",
            "{lbl}"
        }
    }
}

#[component]
fn MenuItem(
    label: &'static str,
    shortcut: Option<&'static str>,
    #[props(default)] destructive: bool,
    onclick: EventHandler<()>,
    icon: Element,
) -> Element {
    let class = if destructive {
        "flex w-full cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-destructive outline-hidden select-none hover:bg-destructive/10"
    } else {
        "flex w-full cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none hover:bg-accent hover:text-accent-foreground"
    };
    rsx! {
        button {
            r#type: "button",
            role: "menuitem",
            class: "{class}",
            onclick: move |_| onclick.call(()),
            {icon}
            span { class: "flex-1 text-left", "{label}" }
            if let Some(sc) = shortcut {
                span { class: "ml-4 text-[11px] text-muted-foreground tabular-nums", "{sc}" }
            }
        }
    }
}

#[component]
fn StudioNodeView(
    ctx: NodeViewCtx<StudioNodeData>,
    read_only: bool,
    on_add_from: EventHandler<(String, String)>,
    on_context: EventHandler<(String, f64, f64)>,
    on_open: EventHandler<String>,
) -> Element {
    let node = ctx.node.clone();
    let ty = node.node_type.clone().unwrap_or_else(|| "unknown".into());
    let data = node.data.clone();
    let id = node.id.clone();
    let label = node.label.clone();
    let selected = node.selected;
    let id_ctx = id.clone();
    let id_open = id.clone();

    if ty == "stickyNote" {
        let sticky_bg = data.color.clone().unwrap_or_else(|| "#fef3c7".into());
        let content = data
            .note
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| label.clone());
        let ring = if selected {
            "ring-2 ring-[var(--brand)] ring-offset-2 ring-offset-[var(--canvas)]"
        } else {
            ""
        };
        let opacity = if data.disabled { "opacity-55" } else { "" };
        return rsx! {
            div {
                class: "relative min-h-[100px] min-w-[160px] max-w-[280px] rounded-lg border border-black/10 px-3 py-2 shadow-md {ring} {opacity}",
                style: "background:{sticky_bg}",
                ondoubleclick: move |e| {
                    e.stop_propagation();
                    on_open.call(id_open.clone());
                },
                oncontextmenu: move |e| {
                    if read_only { return; }
                    e.prevent_default();
                    e.stop_propagation();
                    let c = e.client_coordinates();
                    on_context.call((id_ctx.clone(), c.x, c.y));
                },
                div { class: "whitespace-pre-wrap break-words text-sm leading-snug text-foreground/90",
                    "{content}"
                }
            }
        };
    }

    let trigger = is_trigger_type(&ty);
    let accent = node_accent(&ty);
    let sources = source_handles_for_type(&ty, &data.switch_cases, data.error_output);
    let targets = target_handles_for_type(&ty);
    let multi_out = sources.len() > 1 || sources.first().map(|s| s.as_str()) != Some("main");
    let multi_in = targets.len() > 1;
    let status = data.run_status.clone().unwrap_or_default();
    let box_shape = if trigger {
        "rounded-[28px_12px_12px_28px]"
    } else {
        "rounded-xl"
    };
    // Solid brand ring — faint color-mix halos read as white/grey on dark canvas.
    let selected_shadow = if selected {
        "ring-2 ring-[var(--brand)] ring-offset-2 ring-offset-[var(--canvas)] shadow-[0_4px_16px_rgba(0,0,0,0.18)]"
    } else {
        "shadow-[0_1px_3px_rgba(0,0,0,0.12),0_4px_12px_rgba(0,0,0,0.06)]"
    };
    let status_ring = match status.as_str() {
        "completed" => "ring-2 ring-emerald-500/90 ring-offset-2 ring-offset-[var(--canvas)]",
        "failed" => "ring-2 ring-red-500/90 ring-offset-2 ring-offset-[var(--canvas)]",
        "running" => "ring-2 ring-[var(--brand)] ring-offset-2 ring-offset-[var(--canvas)]",
        _ => "",
    };
    let opacity = if data.disabled { "opacity-55" } else { "" };
    let style = format!(
        "border-color:{accent};color:{accent};background-image:linear-gradient(160deg,color-mix(in srgb,{accent} 14%,var(--card)) 0%,var(--card) 55%);"
    );
    let show_duration =
        data.duration_ms.is_some() && matches!(status.as_str(), "completed" | "failed");
    let http = data.http_method.clone();
    let issue_title = data.issues.join(" · ");

    rsx! {
        div {
            class: "group relative size-[72px] {opacity}",
            ondoubleclick: move |e| {
                e.stop_propagation();
                on_open.call(id_open.clone());
            },
            oncontextmenu: move |e| {
                if read_only { return; }
                e.prevent_default();
                e.stop_propagation();
                let c = e.client_coordinates();
                on_context.call((id_ctx.clone(), c.x, c.y));
            },
            for (i, hid) in targets.iter().enumerate() {
                {
                    let offset = if multi_in {
                        (i as f64 + 1.0) / (targets.len() as f64 + 1.0)
                    } else {
                        0.5
                    };
                    rsx! {
                        Handle {
                            id: Some(hid.clone()),
                            kind: HandleKind::Target,
                            position: Side::Left,
                            offset,
                            class: Some("!size-[11px] !border-2 !border-[var(--canvas)] !bg-muted-foreground !shadow-sm".into()),
                        }
                    }
                }
            }

            div {
                class: "relative flex size-[72px] items-center justify-center border-[2.5px] bg-card {box_shape} {selected_shadow} {status_ring}",
                style: "{style}",
                NodeGlyph { ty: ty.clone(), class: Some("size-7 drop-shadow-sm".into()) }
                if data.has_issues {
                    span {
                        class: "absolute -left-1 -top-1 z-[2] flex size-5 items-center justify-center rounded-full bg-amber-500 text-[11px] font-bold text-white shadow ring-2 ring-[var(--canvas)]",
                        title: "{issue_title}",
                        "!"
                    }
                }
            }

            if status == "completed" {
                span { class: "absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-emerald-500 text-white shadow-md ring-2 ring-[var(--canvas)] text-[10px] font-bold",
                    "✓"
                }
            } else if status == "failed" {
                span { class: "absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-red-500 text-white shadow-md ring-2 ring-[var(--canvas)]",
                    X { class: Some("size-3".into()) }
                }
            } else if status == "running" {
                span { class: "absolute -bottom-0.5 -right-0.5 z-[1] flex size-5 items-center justify-center rounded-full bg-[var(--brand)] text-white shadow-md ring-2 ring-[var(--canvas)] text-[9px]",
                    "…"
                }
            }

            div { class: "pointer-events-none absolute left-1/2 top-[calc(100%+0.5rem)] w-[148px] -translate-x-1/2 px-0.5 text-center",
                div { class: "truncate text-[12px] font-semibold leading-tight tracking-tight text-foreground",
                    "{label}"
                }
                if show_duration {
                    if let Some(ms) = data.duration_ms {
                        div { class: "mt-0.5 text-[10px] tabular-nums text-muted-foreground",
                            "{format_duration_ms(ms)}"
                        }
                    }
                }
                if data.disabled {
                    div { class: "mt-0.5 text-[10px] text-muted-foreground", "deactivated" }
                }
            }

            for (i, hid) in sources.iter().enumerate() {
                {
                    let offset = if multi_out {
                        (i as f64 + 1.0) / (sources.len() as f64 + 1.0)
                    } else {
                        0.5
                    };
                    let is_error = hid == "error";
                    let handle_class = if is_error {
                        "!size-[11px] !border-2 !border-[var(--canvas)] !bg-destructive !shadow-sm"
                    } else {
                        "!size-[11px] !border-2 !border-[var(--canvas)] !bg-muted-foreground !shadow-sm"
                    };
                    let hid_add = hid.clone();
                    let id_add = id.clone();
                    let top_pct = offset * 100.0;
                    rsx! {
                        Handle {
                            id: Some(hid.clone()),
                            kind: HandleKind::Source,
                            position: Side::Right,
                            offset,
                            class: Some(handle_class.into()),
                        }
                        if !read_only {
                            button {
                                r#type: "button",
                                class: "absolute z-10 flex size-6 -translate-y-1/2 items-center justify-center rounded-full border border-border bg-card text-muted-foreground shadow-sm opacity-0 transition-opacity duration-150 group-hover:opacity-100 hover:border-muted-foreground/60 hover:text-foreground",
                                style: "top:{top_pct}%;right:-28px;",
                                title: "Add next step",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    on_add_from.call((id_add.clone(), hid_add.clone()));
                                },
                                Plus { class: Some("size-3".into()) }
                            }
                        }
                    }
                }
            }

            if multi_out {
                div { class: "pointer-events-none absolute -right-1 top-[calc(100%+0.35rem)] flex flex-col items-end gap-0.5 text-[8px] leading-none text-muted-foreground",
                    for h in sources.iter() {
                        {
                            let lbl = source_handle_label(&ty, h, http.as_deref());
                            let cls = if h == "error" { "text-destructive" } else { "" };
                            rsx! { span { class: "{cls}", "{lbl}" } }
                        }
                    }
                }
            }
            if multi_in {
                div { class: "pointer-events-none absolute -left-1 top-[calc(100%+0.35rem)] flex flex-col gap-0.5 text-[8px] leading-none text-muted-foreground",
                    for h in targets.iter() {
                        {
                            let lbl = target_handle_label(&ty, h);
                            rsx! { span { "{lbl}" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn FlowCanvas(
    nodes: Signal<Vec<Node<StudioNodeData>>>,
    edges: Signal<Vec<Edge>>,
    #[props(default)] doc_nodes: Signal<Vec<serde_json::Value>>,
    #[props(default)] run_logs: Signal<Vec<serde_json::Value>>,
    #[props(default = true)] fit_view: bool,
    #[props(default)] read_only: bool,
    #[props(default)] on_node_select: Option<EventHandler<Option<String>>>,
    #[props(default)] on_request_add: Option<EventHandler<PendingAddRequest>>,
    #[props(default)] on_execute_node: Option<EventHandler<String>>,
    #[props(default)] on_toast: Option<EventHandler<String>>,
) -> Element {
    let draggable = !read_only;
    let mut edge_menu = use_signal(|| None::<(String, f64, f64)>);
    let mut node_menu = use_signal(|| None::<(String, f64, f64)>);
    let mut rename_draft = use_signal(|| None::<(String, String)>);
    // ("edge"|"node", id, display name)
    let mut confirm_delete = use_signal(|| None::<(String, String, String)>);

    // Sync doc_nodes when flow deletes nodes (Backspace/Delete).
    use_effect(move || {
        let live: std::collections::HashSet<String> =
            nodes().iter().map(|n| n.id.clone()).collect();
        doc_nodes.with_mut(|dn| {
            dn.retain(|n| {
                n.get("id")
                    .and_then(|v| v.as_str())
                    .map(|id| live.contains(id))
                    .unwrap_or(false)
            });
        });
    });

    // Refresh studio data (issues/http labels) when doc_nodes change.
    use_effect(move || {
        let docs = doc_nodes();
        nodes.with_mut(|ns| studio_data_refresh(ns, &docs));
    });

    use_effect(move || {
        let logs = run_logs();
        let docs = doc_nodes();
        nodes.with_mut(|ns| {
            edges.with_mut(|es| {
                apply_run_overlay(ns, es, &logs, &docs);
            });
        });
    });

    let mut close_menus = move || {
        edge_menu.set(None);
        node_menu.set(None);
    };

    let open_details = {
        let on_node_select = on_node_select.clone();
        EventHandler::new(move |id: String| {
            nodes.with_mut(|ns| {
                for n in ns.iter_mut() {
                    n.selected = n.id == id;
                }
            });
            if let Some(cb) = &on_node_select {
                cb.call(Some(id));
            }
        })
    };

    let mut toggle_disabled = move |id: String| {
        let mut next_disabled = false;
        doc_nodes.with_mut(|docs| {
            if let Some(n) = docs
                .iter_mut()
                .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
            {
                if let Some(obj) = n.as_object_mut() {
                    let mut settings = obj
                        .get("settings")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    if let Some(s) = settings.as_object_mut() {
                        let cur = s.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false);
                        next_disabled = !cur;
                        s.insert("disabled".into(), serde_json::Value::Bool(next_disabled));
                    }
                    obj.insert("settings".into(), settings);
                }
            }
        });
        nodes.with_mut(|ns| {
            if let Some(n) = ns.iter_mut().find(|n| n.id == id) {
                n.data.disabled = next_disabled;
                // Faded/deactivated nodes stay editable via double-click, but locked in place.
                n.draggable = !next_disabled;
            }
        });
        if let Some(cb) = &on_toast {
            cb.call(if next_disabled {
                "Node deactivated".into()
            } else {
                "Node activated".into()
            });
        }
    };

    let mut duplicate_node = move |id: String| {
        let Some(src_doc) = doc_nodes()
            .into_iter()
            .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        else {
            return;
        };
        let Some(src_flow) = nodes().into_iter().find(|n| n.id == id) else {
            return;
        };
        let new_id = format!(
            "n_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| format!("{:x}", d.as_millis()))
                .unwrap_or_else(|_| "dup".into())
        );
        let mut copy = src_doc.clone();
        if let Some(obj) = copy.as_object_mut() {
            obj.insert("id".into(), serde_json::Value::String(new_id.clone()));
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Node");
            obj.insert(
                "name".into(),
                serde_json::Value::String(format!("{name} copy")),
            );
            obj.insert(
                "position".into(),
                serde_json::json!({
                    "x": src_flow.position.x + 48.0,
                    "y": src_flow.position.y + 48.0,
                }),
            );
        }
        let mut flow = src_flow.clone();
        flow.id = new_id.clone();
        flow.label = format!("{} copy", src_flow.label);
        flow.position = Point::new(src_flow.position.x + 48.0, src_flow.position.y + 48.0);
        flow.selected = true;
        doc_nodes.with_mut(|d| d.push(copy));
        nodes.with_mut(|ns| {
            for n in ns.iter_mut() {
                n.selected = false;
            }
            ns.push(flow);
        });
        if let Some(cb) = &on_node_select {
            cb.call(Some(new_id));
        }
        if let Some(cb) = &on_toast {
            cb.call("Node duplicated".into());
        }
    };

    let mut delete_node = move |id: String| {
        nodes.with_mut(|ns| ns.retain(|n| n.id != id));
        edges.with_mut(|es| es.retain(|e| e.source != id && e.target != id));
        doc_nodes.with_mut(|dn| {
            dn.retain(|n| n.get("id").and_then(|v| v.as_str()) != Some(id.as_str()))
        });
        if let Some(cb) = &on_node_select {
            cb.call(None);
        }
    };

    let mut copy_node = move |id: String| {
        let Some(doc) = doc_nodes()
            .into_iter()
            .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        else {
            return;
        };
        let payload = serde_json::json!({
            "kind": "botflow-node",
            "node": {
                "type": doc.get("type"),
                "name": doc.get("name"),
                "parameters": doc.get("parameters"),
                "settings": doc.get("settings"),
            }
        });
        let text = payload.to_string().replace('`', "\\`").replace('\\', "\\\\");
        let _ = document::eval(&format!(
            "navigator.clipboard.writeText(`{text}`).catch(()=>{{}})"
        ));
        if let Some(cb) = &on_toast {
            cb.call("Node copied".into());
        }
    };

    let on_add_from = {
        let on_request_add = on_request_add.clone();
        EventHandler::new(move |(id, handle): (String, String)| {
            if let Some(cb) = &on_request_add {
                cb.call(PendingAddRequest {
                    connect_from: Some(id),
                    source_handle: Some(handle),
                    triggers_only: false,
                });
            }
        })
    };

    let on_context = EventHandler::new(move |(id, x, y): (String, f64, f64)| {
        edge_menu.set(None);
        nodes.with_mut(|ns| {
            for n in ns.iter_mut() {
                n.selected = n.id == id;
            }
        });
        node_menu.set(Some((id, x, y)));
    });

    let on_open_node = open_details.clone();

    let node_view = {
        let on_add_from = on_add_from.clone();
        let on_context = on_context.clone();
        let on_open_node = on_open_node.clone();
        Callback::new(move |ctx: NodeViewCtx<StudioNodeData>| {
            rsx! {
                StudioNodeView {
                    ctx,
                    read_only,
                    on_add_from: on_add_from.clone(),
                    on_context: on_context.clone(),
                    on_open: on_open_node.clone(),
                }
            }
        })
    };

    let edge_view = {
        let mut edges = edges;
        let on_request_add = on_request_add.clone();
        let mut confirm_delete = confirm_delete;
        Callback::new(move |ctx: EdgeViewCtx| {
            let edge = ctx.edge.clone();
            let d = ctx.path.d.clone();
            let lx = ctx.path.label.x;
            let ly = ctx.path.label.y;
            let marker = ctx.marker_end.clone().unwrap_or_default();
            let style = edge
                .style
                .clone()
                .unwrap_or_else(|| {
                    "stroke: var(--canvas-edge, var(--df-edge)); stroke-width: 1.75;".into()
                });
            let label = edge.label.clone();
            let edge_id = edge.id.clone();
            let source = edge.source.clone();
            let source_handle = edge.source_handle.clone();
            let edge_id_ctx = edge.id.clone();
            rsx! {
                path {
                    class: "df-edge-path",
                    d: "{d}",
                    style: "{style}",
                    fill: "none",
                    "marker-end": "{marker}",
                }
                foreignObject {
                    x: lx - 52.0,
                    y: ly - 14.0,
                    width: 104.0,
                    height: 28.0,
                    div {
                        class: "flex h-full items-center justify-center gap-1",
                        style: "pointer-events: auto;",
                        oncontextmenu: move |e| {
                            if read_only { return; }
                            e.prevent_default();
                            e.stop_propagation();
                            let c = e.client_coordinates();
                            node_menu.set(None);
                            edge_menu.set(Some((edge_id_ctx.clone(), c.x, c.y)));
                        },
                        if let Some(lbl) = label.clone() {
                            {edge_label_chip(&lbl, &style)}
                        }
                        if !read_only {
                            button {
                                r#type: "button",
                                class: "flex h-5 w-5 items-center justify-center rounded-full border bg-card text-muted-foreground shadow-sm hover:border-[var(--brand)] hover:text-foreground",
                                title: "Add node",
                                onclick: {
                                    let on_request_add = on_request_add.clone();
                                    let source = source.clone();
                                    let source_handle = source_handle.clone();
                                    move |e: Event<MouseData>| {
                                        e.stop_propagation();
                                        if let Some(cb) = &on_request_add {
                                            cb.call(PendingAddRequest {
                                                connect_from: Some(source.clone()),
                                                source_handle: source_handle.clone(),
                                                triggers_only: false,
                                            });
                                        }
                                    }
                                },
                                Plus { class: Some("size-3".into()) }
                            }
                            button {
                                r#type: "button",
                                class: "flex h-5 w-5 items-center justify-center rounded-full border bg-card text-muted-foreground shadow-sm hover:border-destructive hover:bg-destructive/10 hover:text-destructive",
                                title: "Delete connection",
                                onclick: {
                                    let edge_id = edge_id.clone();
                                    move |e: Event<MouseData>| {
                                        e.stop_propagation();
                                        confirm_delete.set(Some((
                                            "edge".into(),
                                            edge_id.clone(),
                                            "this connection".into(),
                                        )));
                                    }
                                },
                                Trash2 { class: Some("size-3".into()) }
                            }
                        }
                    }
                }
            }
        })
    };

    let empty = nodes().is_empty();
    let disabled_selected = node_menu().as_ref().and_then(|(id, _, _)| {
        nodes()
            .into_iter()
            .find(|n| n.id == *id)
            .map(|n| n.data.disabled)
    });

    rsx! {
        div {
            class: "relative h-full w-full min-h-0 bg-[var(--canvas)] outline-none",
            style: "--df-bg: var(--canvas); --df-edge: var(--canvas-edge); --df-grid: var(--canvas-dot); --df-accent: var(--brand); --df-node-bg: var(--card); --df-node-color: var(--foreground); --df-node-border: var(--canvas-node-border); --df-panel-bg: var(--card); --df-panel-border: var(--border); --df-panel-color: var(--muted-foreground);",
            tabindex: "0",
            onkeydown: move |e| {
                if read_only { return; }
                let key = e.key();
                let mod_key = e.modifiers().ctrl() || e.modifiers().meta();
                let selected: Vec<String> = nodes()
                    .into_iter()
                    .filter(|n| n.selected)
                    .map(|n| n.id.clone())
                    .collect();

                if mod_key && matches!(key, Key::Character(ref c) if c.eq_ignore_ascii_case("a")) {
                    e.prevent_default();
                    nodes.with_mut(|ns| {
                        for n in ns.iter_mut() {
                            n.selected = true;
                        }
                    });
                    return;
                }
                if selected.is_empty() {
                    return;
                }
                let id = selected[0].clone();
                match key {
                    Key::Enter if !mod_key => {
                        e.prevent_default();
                        close_menus();
                        open_details.call(id);
                    }
                    Key::Character(ref c) if c == " " && !mod_key && selected.len() == 1 => {
                        e.prevent_default();
                        close_menus();
                        let name = nodes()
                            .into_iter()
                            .find(|n| n.id == id)
                            .map(|n| n.label)
                            .unwrap_or_default();
                        rename_draft.set(Some((id, name)));
                    }
                    Key::Character(ref c)
                        if c.eq_ignore_ascii_case("d") && !mod_key && selected.len() == 1 =>
                    {
                        e.prevent_default();
                        toggle_disabled(id);
                    }
                    Key::Character(ref c)
                        if c.eq_ignore_ascii_case("d") && mod_key && selected.len() == 1 =>
                    {
                        e.prevent_default();
                        duplicate_node(id);
                    }
                    Key::Character(ref c)
                        if c.eq_ignore_ascii_case("c") && mod_key && selected.len() == 1 =>
                    {
                        e.prevent_default();
                        copy_node(id);
                    }
                    _ => {}
                }
            },
            Flow {
                nodes,
                edges,
                fit_view,
                // Cap at 1× so fit_view never CSS-scales nodes up — WebView2
                // blurs text/icons at fractional zooms above 100%.
                max_zoom: 1.0,
                min_zoom: 0.25,
                nodes_draggable: draggable,
                delete_key: !read_only,
                class: Some("h-full w-full".to_string()),
                node_view: Some(node_view),
                edge_view: Some(edge_view),
                on_connect_end: move |end: ConnectEnd| {
                    if read_only { return; }
                    if end.connection.is_some() { return; }
                    if let Some(cb) = &on_request_add {
                        cb.call(PendingAddRequest {
                            connect_from: None,
                            source_handle: None,
                            triggers_only: false,
                        });
                    }
                },
                on_node_click: move |id: String| {
                    close_menus();
                    // Select only — editor opens on double-click (node view / pane).
                    nodes.with_mut(|ns| {
                        for n in ns.iter_mut() {
                            n.selected = n.id == id;
                        }
                    });
                },
                on_edge_click: move |id: String| {
                    node_menu.set(None);
                    edge_menu.set(Some((id, 24.0, 24.0)));
                },
                on_pane_click: move |_| {
                    close_menus();
                    if let Some(cb) = &on_node_select {
                        cb.call(None);
                    }
                },
                on_pane_double_click: move |pt: Point| {
                    if read_only { return; }
                    // dioxus-flow fires pane dblclick for any surface hit; ignore
                    // when the pointer is over a node (node view also stops bubble).
                    let over_node = nodes().into_iter().any(|n| {
                        let w = n.measured.map(|m| m.width).unwrap_or(72.0);
                        let h = n.measured.map(|m| m.height).unwrap_or(72.0);
                        pt.x >= n.position.x
                            && pt.x <= n.position.x + w
                            && pt.y >= n.position.y
                            && pt.y <= n.position.y + h
                    });
                    if over_node {
                        if let Some(id) = nodes()
                            .into_iter()
                            .find(|n| n.selected)
                            .map(|n| n.id.clone())
                        {
                            open_details.call(id);
                        }
                        return;
                    }
                    if let Some(cb) = &on_request_add {
                        cb.call(PendingAddRequest {
                            connect_from: None,
                            source_handle: None,
                            triggers_only: false,
                        });
                    }
                },
                Background { variant: BackgroundVariant::Dots, gap: 18.0, size: 1.25 }
                Controls {}
                MiniMap {}
            }

            if empty && !read_only {
                div { class: "pointer-events-none absolute inset-0 z-10 flex items-center justify-center",
                    div { class: "pointer-events-auto flex flex-col items-center gap-3 rounded-xl border bg-card/95 px-8 py-6 shadow-lg backdrop-blur",
                        p { class: "text-sm text-muted-foreground", "This workflow is empty" }
                        button {
                            class: "inline-flex h-10 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground",
                            onclick: move |_| {
                                if let Some(cb) = &on_request_add {
                                    cb.call(PendingAddRequest {
                                        connect_from: None,
                                        source_handle: None,
                                        triggers_only: true,
                                    });
                                }
                            },
                            Plus { class: Some("h-4 w-4".into()) }
                            "Add first step"
                        }
                    }
                }
            }

            if edge_menu().is_some() || node_menu().is_some() {
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| close_menus(),
                    oncontextmenu: move |e| {
                        e.prevent_default();
                        close_menus();
                    },
                }
            }

            if let Some((eid, x, y)) = edge_menu() {
                {
                    let (left, top) = clamp_menu(x, y);
                    rsx! {
                        div {
                            role: "menu",
                            class: "fixed z-50 min-w-44 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
                            style: "left:{left}px;top:{top}px;",
                            MenuItem {
                                label: "Delete connection",
                                shortcut: None::<&'static str>,
                                destructive: true,
                                onclick: move |_| {
                                    confirm_delete.set(Some((
                                        "edge".into(),
                                        eid.clone(),
                                        "this connection".into(),
                                    )));
                                    close_menus();
                                },
                                icon: rsx! { Trash2 { class: Some("size-4 shrink-0".into()) } },
                            }
                        }
                    }
                }
            }

            if let Some((nid, x, y)) = node_menu() {
                {
                    let (left, top) = clamp_menu(x, y);
                    let disabled = disabled_selected.unwrap_or(false);
                    let activate_label = if disabled { "Activate" } else { "Deactivate" };
                    let nid_open = nid.clone();
                    let nid_exec = nid.clone();
                    let nid_rename = nid.clone();
                    let nid_toggle = nid.clone();
                    let nid_copy = nid.clone();
                    let nid_dup = nid.clone();
                    let nid_del = nid.clone();
                    rsx! {
                        div {
                            role: "menu",
                            class: "fixed z-50 min-w-56 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
                            style: "left:{left}px;top:{top}px;",
                            MenuItem {
                                label: "Open...",
                                shortcut: Some("↵"),
                                onclick: move |_| {
                                    close_menus();
                                    open_details.call(nid_open.clone());
                                },
                                icon: rsx! { Pencil { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            MenuItem {
                                label: "Execute step",
                                shortcut: None::<&'static str>,
                                onclick: move |_| {
                                    close_menus();
                                    if let Some(cb) = &on_execute_node {
                                        cb.call(nid_exec.clone());
                                    }
                                },
                                icon: rsx! { Play { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            div { role: "separator", class: "-mx-1 my-1 h-px bg-border" }
                            MenuItem {
                                label: "Rename",
                                shortcut: Some("Space"),
                                onclick: move |_| {
                                    let id = nid_rename.clone();
                                    let name = nodes()
                                        .into_iter()
                                        .find(|n| n.id == id)
                                        .map(|n| n.label)
                                        .unwrap_or_default();
                                    close_menus();
                                    rename_draft.set(Some((id, name)));
                                },
                                icon: rsx! { Pencil { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            MenuItem {
                                label: activate_label,
                                shortcut: Some("D"),
                                onclick: move |_| {
                                    toggle_disabled(nid_toggle.clone());
                                    close_menus();
                                },
                                icon: rsx! { Play { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            div { role: "separator", class: "-mx-1 my-1 h-px bg-border" }
                            MenuItem {
                                label: "Copy",
                                shortcut: Some("Ctrl+C"),
                                onclick: move |_| {
                                    copy_node(nid_copy.clone());
                                    close_menus();
                                },
                                icon: rsx! { Copy { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            MenuItem {
                                label: "Duplicate",
                                shortcut: Some("Ctrl+D"),
                                onclick: move |_| {
                                    duplicate_node(nid_dup.clone());
                                    close_menus();
                                },
                                icon: rsx! { Copy { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            div { role: "separator", class: "-mx-1 my-1 h-px bg-border" }
                            MenuItem {
                                label: "Select all",
                                shortcut: Some("Ctrl+A"),
                                onclick: move |_| {
                                    nodes.with_mut(|ns| {
                                        for n in ns.iter_mut() {
                                            n.selected = true;
                                        }
                                    });
                                    close_menus();
                                },
                                icon: rsx! { Plus { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            MenuItem {
                                label: "Clear selection",
                                shortcut: None::<&'static str>,
                                onclick: move |_| {
                                    nodes.with_mut(|ns| {
                                        for n in ns.iter_mut() {
                                            n.selected = false;
                                        }
                                    });
                                    if let Some(cb) = &on_node_select {
                                        cb.call(None);
                                    }
                                    close_menus();
                                },
                                icon: rsx! { X { class: Some("size-4 shrink-0 opacity-80".into()) } },
                            }
                            div { role: "separator", class: "-mx-1 my-1 h-px bg-border" }
                            MenuItem {
                                label: "Delete",
                                shortcut: Some("Del"),
                                destructive: true,
                                onclick: move |_| {
                                    let label = nodes()
                                        .into_iter()
                                        .find(|n| n.id == nid_del)
                                        .map(|n| n.label.clone())
                                        .unwrap_or_else(|| nid_del.clone());
                                    confirm_delete.set(Some((
                                        "node".into(),
                                        nid_del.clone(),
                                        label,
                                    )));
                                    close_menus();
                                },
                                icon: rsx! { Trash2 { class: Some("size-4 shrink-0".into()) } },
                            }
                        }
                    }
                }
            }

            if let Some((kind, del_id, del_name)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let kind_confirm = kind.clone();
                    let is_edge = kind == "edge";
                    let title = if is_edge {
                        "Delete connection?".to_string()
                    } else {
                        "Delete node?".to_string()
                    };
                    let description = if is_edge {
                        "This will permanently remove this connection. This action cannot be undone."
                            .to_string()
                    } else {
                        format!(
                            "This will permanently remove node \"{del_name}\" from the workflow. This action cannot be undone."
                        )
                    };
                    rsx! {
                        ConfirmDeleteDialog {
                            title,
                            description,
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                if kind_confirm == "edge" {
                                    edges.with_mut(|es| es.retain(|e| e.id != del_id_confirm));
                                } else {
                                    delete_node(del_id_confirm.clone());
                                }
                                confirm_delete.set(None);
                            },
                        }
                    }
                }
            }

            if let Some((rid, name)) = rename_draft() {
                {
                    let rid_input = rid.clone();
                    let rid_key = rid.clone();
                    let rid_btn = rid.clone();
                    let name_key = name.clone();
                    let name_btn = name.clone();
                    rsx! {
                        div {
                            class: "fixed inset-0 z-[60] flex items-center justify-center bg-black/40 p-4",
                            onclick: move |_| rename_draft.set(None),
                            div {
                                class: "w-full max-w-sm rounded-xl border bg-card p-4 shadow-xl",
                                onclick: move |e| e.stop_propagation(),
                                h3 { class: "text-sm font-semibold", "Rename node" }
                                input {
                                    class: "mt-3 flex h-9 w-full rounded-md border bg-background px-3 text-sm",
                                    value: "{name}",
                                    oninput: move |e| {
                                        rename_draft.set(Some((rid_input.clone(), e.value())));
                                    },
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter {
                                            let trimmed = name_key.trim().to_string();
                                            if !trimmed.is_empty() {
                                                let id = rid_key.clone();
                                                nodes.with_mut(|ns| {
                                                    if let Some(n) =
                                                        ns.iter_mut().find(|n| n.id == id)
                                                    {
                                                        n.label = trimmed.clone();
                                                    }
                                                });
                                                doc_nodes.with_mut(|docs| {
                                                    if let Some(n) = docs.iter_mut().find(|n| {
                                                        n.get("id").and_then(|v| v.as_str())
                                                            == Some(id.as_str())
                                                    }) {
                                                        if let Some(obj) = n.as_object_mut() {
                                                            obj.insert(
                                                                "name".into(),
                                                                serde_json::Value::String(
                                                                    trimmed.clone(),
                                                                ),
                                                            );
                                                        }
                                                    }
                                                });
                                                if let Some(cb) = &on_toast {
                                                    cb.call("Node renamed".into());
                                                }
                                            }
                                            rename_draft.set(None);
                                        }
                                        if e.key() == Key::Escape {
                                            rename_draft.set(None);
                                        }
                                    },
                                }
                                div { class: "mt-3 flex justify-end gap-2",
                                    button {
                                        class: "rounded-md px-3 py-1.5 text-xs hover:bg-muted",
                                        onclick: move |_| rename_draft.set(None),
                                        "Cancel"
                                    }
                                    button {
                                        class: "rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground",
                                        onclick: move |_| {
                                            let trimmed = name_btn.trim().to_string();
                                            if !trimmed.is_empty() {
                                                let id = rid_btn.clone();
                                                nodes.with_mut(|ns| {
                                                    if let Some(n) =
                                                        ns.iter_mut().find(|n| n.id == id)
                                                    {
                                                        n.label = trimmed.clone();
                                                    }
                                                });
                                                doc_nodes.with_mut(|docs| {
                                                    if let Some(n) = docs.iter_mut().find(|n| {
                                                        n.get("id").and_then(|v| v.as_str())
                                                            == Some(id.as_str())
                                                    }) {
                                                        if let Some(obj) = n.as_object_mut() {
                                                            obj.insert(
                                                                "name".into(),
                                                                serde_json::Value::String(trimmed),
                                                            );
                                                        }
                                                    }
                                                });
                                                if let Some(cb) = &on_toast {
                                                    cb.call("Node renamed".into());
                                                }
                                            }
                                            rename_draft.set(None);
                                        },
                                        "Rename"
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
