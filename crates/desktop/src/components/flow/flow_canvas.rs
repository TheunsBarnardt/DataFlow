use dioxus_flow::prelude::{Edge, MarkerKind, Node, Point, Side, Size};

use super::config_issues::get_node_config_issues;
use super::handles::connection_label_for_node;
use crate::components::node_schemas::merge_defaults;

/// Icon-box footprint — must match `size-[72px]` in `StudioNodeView`.
/// Without an explicit size, dioxus-flow falls back to 150×40 and edges
/// anchor ~16px above the visible left/right ports.
const WORKFLOW_NODE_SIZE: Size = Size {
    width: 72.0,
    height: 72.0,
};

/// Canvas node payload — mirrors React `WorkflowCanvasNodeData` (minus JS handlers).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct StudioNodeData {
    pub disabled: bool,
    pub note: Option<String>,
    pub color: Option<String>,
    pub switch_cases: Vec<String>,
    pub error_output: bool,
    pub run_status: Option<String>,
    pub duration_ms: Option<f64>,
    pub http_method: Option<String>,
    pub has_issues: bool,
    pub issues: Vec<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PendingAddRequest {
    pub connect_from: Option<String>,
    pub source_handle: Option<String>,
    pub triggers_only: bool,
}

fn switch_handle_ids(n: &serde_json::Value) -> Vec<String> {
    let ty = n.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if ty != "switch" {
        return Vec::new();
    }
    let params = n.get("parameters").cloned().unwrap_or(serde_json::json!({}));
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("rules");
    if mode == "rules" {
        if let Some(rules) = params.get("rules").and_then(|v| v.as_array()) {
            let mut keys: Vec<String> = rules
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    r.get("outputKey")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| format!("output{i}"))
                })
                .filter(|s| !s.is_empty())
                .collect();
            let fallback = params
                .get("fallbackOutput")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            if !keys.iter().any(|k| k == &fallback) {
                keys.push(fallback);
            }
            return keys;
        }
    }
    if let Some(cases) = params.get("cases").and_then(|v| v.as_array()) {
        return cases
            .iter()
            .filter_map(|c| c.as_str().map(String::from))
            .collect();
    }
    Vec::new()
}

fn studio_data_from_json(n: &serde_json::Value) -> StudioNodeData {
    let ty = n.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let settings = n.get("settings").cloned().unwrap_or(serde_json::json!({}));
    let params = n.get("parameters").cloned().unwrap_or(serde_json::json!({}));
    let disabled = settings
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let note = if ty == "stickyNote" {
        params
            .get("content")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                settings
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
    } else if settings
        .get("displayNoteInFlow")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        settings
            .get("notes")
            .and_then(|v| v.as_str())
            .map(String::from)
    } else {
        None
    };
    let color = if ty == "stickyNote" {
        params
            .get("color")
            .and_then(|v| v.as_str())
            .map(String::from)
    } else {
        None
    };
    let http_method = connection_label_for_node(ty, &params);
    let issues = get_node_config_issues(ty, &params, disabled);
    let has_issues = !issues.is_empty();
    StudioNodeData {
        disabled,
        note,
        color,
        switch_cases: switch_handle_ids(n),
        error_output: settings
            .get("onError")
            .and_then(|v| v.as_str())
            == Some("continueErrorOutput"),
        run_status: None,
        duration_ms: None,
        http_method,
        has_issues,
        issues,
    }
}

fn make_flow_node(n: &serde_json::Value) -> Option<Node<StudioNodeData>> {
    let id = n
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| n.get("_id").and_then(|v| v.as_str()).map(String::from))?;
    let label = n
        .get("name")
        .or_else(|| n.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let x = n
        .get("position")
        .and_then(|p| p.get("x"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let y = n
        .get("position")
        .and_then(|p| p.get("y"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let node_type = n.get("type").and_then(|v| v.as_str()).map(String::from);
    let data = studio_data_from_json(n);
    // Deactivated (faded) nodes stay selectable/editable but are not dragged.
    let can_drag = !data.disabled;
    let mut node = Node::with_data(id, label, Point::new(x, y), data)
        .sides(Side::Left, Side::Right)
        .draggable(can_drag);
    if let Some(ty) = node_type {
        // Sticky notes size to content; icon nodes must declare 72×72 so
        // edge anchors hit the handle centers (not DEFAULT_NODE_SIZE).
        if ty != "stickyNote" {
            node = node.size(WORKFLOW_NODE_SIZE);
        }
        node = node.node_type(ty);
    } else {
        node = node.size(WORKFLOW_NODE_SIZE);
    }
    Some(node)
}

fn make_edge_from_json(
    e: &serde_json::Value,
    nodes_by_id: &std::collections::HashMap<&str, &serde_json::Value>,
) -> Option<Edge> {
    let source = e.get("source").and_then(|v| v.as_str())?;
    let target = e.get("target").and_then(|v| v.as_str())?;
    let mut edge = Edge::new(source, target)
        .marker_end(MarkerKind::ArrowClosed)
        .style("stroke: var(--canvas-edge, var(--df-edge)); stroke-width: 1.75;");
    if let Some(id) = e.get("id").and_then(|v| v.as_str()) {
        edge = edge.id(id);
    }
    // Seeded / React edges often omit handles; custom nodes register "main".
    // Matching that id keeps anchors on Left/Right instead of side fallbacks.
    let source_handle = e
        .get("sourceHandle")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("main");
    let target_handle = e
        .get("targetHandle")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("main");
    edge = edge.source_handle(source_handle).target_handle(target_handle);
    if let Some(src) = nodes_by_id.get(source) {
        let ty = src.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let params = src
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        if let Some(label) = connection_label_for_node(ty, &params) {
            edge = edge.label(label);
        }
    }
    edge.selectable = false;
    Some(edge)
}

pub fn workflow_json_to_flow(
    nodes: &[serde_json::Value],
    edges: &[serde_json::Value],
) -> (Vec<Node<StudioNodeData>>, Vec<Edge>) {
    let flow_nodes: Vec<Node<StudioNodeData>> = nodes.iter().filter_map(make_flow_node).collect();
    let nodes_by_id: std::collections::HashMap<&str, &serde_json::Value> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(|id| (id, n)))
        .collect();
    let flow_edges: Vec<Edge> = edges
        .iter()
        .filter_map(|e| make_edge_from_json(e, &nodes_by_id))
        .collect();
    (flow_nodes, flow_edges)
}

/// Sync canvas geometry back into workflow JSON nodes/edges (preserves parameters).
pub fn merge_flow_into_workflow_json(
    mut nodes: Vec<serde_json::Value>,
    _prev_edges: Vec<serde_json::Value>,
    flow_nodes: &[Node<StudioNodeData>],
    flow_edges: &[Edge],
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    for fn_node in flow_nodes {
        let id = fn_node.id.as_str();
        if let Some(raw) = nodes
            .iter_mut()
            .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id))
        {
            if let Some(obj) = raw.as_object_mut() {
                obj.insert(
                    "position".into(),
                    serde_json::json!({ "x": fn_node.position.x, "y": fn_node.position.y }),
                );
                obj.insert(
                    "name".into(),
                    serde_json::Value::String(fn_node.label.clone()),
                );
                if let Some(ty) = &fn_node.node_type {
                    obj.insert("type".into(), serde_json::Value::String(ty.clone()));
                }
                if fn_node.data.disabled {
                    let mut settings = obj
                        .get("settings")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    if let Some(s) = settings.as_object_mut() {
                        s.insert("disabled".into(), serde_json::Value::Bool(true));
                    }
                    obj.insert("settings".into(), settings);
                }
            }
        } else {
            nodes.push(serde_json::json!({
                "id": id,
                "name": fn_node.label,
                "type": fn_node.node_type.clone().unwrap_or_else(|| "unknown".into()),
                "position": { "x": fn_node.position.x, "y": fn_node.position.y },
                "parameters": {},
            }));
        }
    }

    let live: std::collections::HashSet<&str> =
        flow_nodes.iter().map(|n| n.id.as_str()).collect();
    nodes.retain(|n| {
        n.get("id")
            .and_then(|v| v.as_str())
            .map(|id| live.contains(id))
            .unwrap_or(false)
    });

    let edges = flow_edges
        .iter()
        .map(|e| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".into(), serde_json::Value::String(e.id.clone()));
            obj.insert("source".into(), serde_json::Value::String(e.source.clone()));
            obj.insert("target".into(), serde_json::Value::String(e.target.clone()));
            if let Some(h) = &e.source_handle {
                obj.insert("sourceHandle".into(), serde_json::Value::String(h.clone()));
            }
            if let Some(h) = &e.target_handle {
                obj.insert("targetHandle".into(), serde_json::Value::String(h.clone()));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    (nodes, edges)
}

pub fn new_catalog_node(
    item_type: &str,
    label: &str,
    index: usize,
    connect_from: Option<&str>,
    source_handle: Option<&str>,
    source_pos: Option<(f64, f64)>,
) -> (serde_json::Value, Node<StudioNodeData>, Option<Edge>) {
    let id = format!("n_{}", uuid_ish(index));
    let (x, y) = if let Some((sx, sy)) = source_pos {
        (sx + 240.0, sy)
    } else {
        (
            120.0 + (index as f64 % 6.0) * 180.0,
            140.0 + ((index / 6) as f64) * 140.0,
        )
    };
    let mut parameters = merge_defaults(item_type, &serde_json::json!({}));
    if item_type == "webhook" {
        if parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("path".into(), serde_json::Value::String(uuid_ish(index + 7)));
            }
        }
        if parameters.get("method").is_none() {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("method".into(), serde_json::json!("POST"));
            }
        }
        if parameters.get("authentication").is_none() {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("authentication".into(), serde_json::json!("none"));
            }
        }
        if parameters.get("respond").is_none() {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("respond".into(), serde_json::json!("immediately"));
            }
        }
    } else if item_type == "apiEndpoint" {
        let path_empty = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty();
        if path_empty {
            if let Some(obj) = parameters.as_object_mut() {
                let salt = uuid_ish(index + 3);
                let short = &salt[..6.min(salt.len())];
                obj.insert(
                    "path".into(),
                    serde_json::Value::String(format!("endpoint-{short}")),
                );
            }
        }
        if parameters.get("method").is_none() {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("method".into(), serde_json::json!("GET"));
            }
        }
        if parameters.get("auth").is_none() {
            if let Some(obj) = parameters.as_object_mut() {
                obj.insert("auth".into(), serde_json::json!("jwt"));
            }
        }
    } else if item_type == "stickyNote" {
        parameters = serde_json::json!({
            "content": "Sticky note",
            "color": "#fef3c7",
        });
    }
    let raw = serde_json::json!({
        "id": id,
        "name": label,
        "type": item_type,
        "position": { "x": x, "y": y },
        "parameters": parameters,
    });
    let data = studio_data_from_json(&raw);
    let mut flow = Node::with_data(id.clone(), label.to_string(), Point::new(x, y), data)
        .node_type(item_type.to_string())
        .sides(Side::Left, Side::Right);
    if item_type != "stickyNote" {
        flow = flow.size(WORKFLOW_NODE_SIZE);
    }
    let edge = connect_from.map(|src| {
        let handle = source_handle.unwrap_or("main");
        let mut e = Edge::new(src, &id)
            .id(format!("e_{src}_{id}_{handle}"))
            .source_handle(handle)
            .marker_end(MarkerKind::ArrowClosed)
            .style("stroke: var(--canvas-edge, var(--df-edge)); stroke-width: 1.75;");
        e.selectable = false;
        e
    });
    (raw, flow, edge)
}

fn uuid_ish(salt: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{ms:x}_{salt:x}")
}

fn format_duration_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{}ms", ms.round() as i64)
    } else {
        format!("{:.1}s", ms / 1000.0)
    }
}

fn item_count_from_output(output: Option<&serde_json::Value>, source_handle: Option<&str>) -> Option<usize> {
    let output = output?;
    // Prefer array length; for multi-output maps try handle key.
    if let Some(h) = source_handle {
        if let Some(by_handle) = output.get(h) {
            if let Some(arr) = by_handle.as_array() {
                return Some(arr.len());
            }
            if let Some(items) = by_handle.get("json").and_then(|v| v.as_array()) {
                return Some(items.len());
            }
        }
    }
    if let Some(arr) = output.as_array() {
        return Some(arr.len());
    }
    if let Some(arr) = output.get("json").and_then(|v| v.as_array()) {
        return Some(arr.len());
    }
    if let Some(main) = output.get("main").and_then(|v| v.as_array()) {
        return Some(main.len());
    }
    None
}

/// Apply execution overlays onto flow nodes/edges (React `buildNodeOverlayMap` parity).
pub fn apply_run_overlay(
    flow_nodes: &mut [Node<StudioNodeData>],
    flow_edges: &mut [Edge],
    run_logs: &[serde_json::Value],
    doc_nodes: &[serde_json::Value],
) {
    let mut by_id: std::collections::HashMap<String, &serde_json::Value> =
        std::collections::HashMap::new();
    for log in run_logs {
        if let Some(id) = log.get("nodeId").and_then(|v| v.as_str()) {
            by_id.insert(id.to_string(), log);
        }
    }
    for node in flow_nodes.iter_mut() {
        if let Some(log) = by_id.get(&node.id) {
            node.data.run_status = log
                .get("status")
                .and_then(|v| v.as_str())
                .map(String::from);
            node.data.duration_ms = log
                .get("durationMs")
                .and_then(|v| v.as_f64())
                .or_else(|| log.get("duration").and_then(|v| v.as_f64()));
        } else {
            node.data.run_status = None;
            node.data.duration_ms = None;
        }
    }

    let doc_by_id: std::collections::HashMap<&str, &serde_json::Value> = doc_nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(|id| (id, n)))
        .collect();

    for edge in flow_edges.iter_mut() {
        let overlay = by_id.get(&edge.source);
        let executed = overlay.is_some();
        let failed = overlay
            .and_then(|l| l.get("status").and_then(|v| v.as_str()))
            == Some("failed");
        let item_count = overlay.and_then(|l| {
            item_count_from_output(l.get("output"), edge.source_handle.as_deref())
        });
        let stroke = if failed {
            "var(--destructive)"
        } else if executed {
            "var(--brand)"
        } else {
            "var(--canvas-edge)"
        };
        let width = if executed { "2" } else { "1.5" };
        edge.style = Some(format!("stroke: {stroke}; stroke-width: {width};"));
        edge.animated = overlay
            .and_then(|l| l.get("status").and_then(|v| v.as_str()))
            == Some("running");

        if let Some(count) = item_count {
            let label = if count == 1 {
                "1 item".to_string()
            } else {
                format!("{count} items")
            };
            edge.label = Some(label);
        } else if let Some(src) = doc_by_id.get(edge.source.as_str()) {
            let ty = src.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let params = src
                .get("parameters")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            edge.label = connection_label_for_node(ty, &params);
        }
    }
}

/// Recompute issues / labels from JSON without wiping run overlays.
pub fn studio_data_refresh(nodes: &mut [Node<StudioNodeData>], doc_nodes: &[serde_json::Value]) {
    for node in nodes.iter_mut() {
        if let Some(doc) = doc_nodes.iter().find(|n| {
            n.get("id").and_then(|v| v.as_str()) == Some(node.id.as_str())
        }) {
            let run_status = node.data.run_status.clone();
            let duration_ms = node.data.duration_ms;
            let mut data = studio_data_from_json(doc);
            data.run_status = run_status;
            data.duration_ms = duration_ms;
            node.draggable = !data.disabled;
            node.data = data;
        }
    }
}
