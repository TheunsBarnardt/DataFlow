use std::sync::Mutex;

use dioxus::prelude::*;

use crate::components::icons::Zap;
use crate::components::panels::json_string;
use crate::components::ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, PageLoader,
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NodesPanelVariant {
    #[default]
    List,
    Grid,
}

#[derive(Clone, PartialEq)]
pub struct NodeCatalogItem {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub description: String,
}

const RECENT_LIMIT: usize = 8;
static RECENT_TYPES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn read_recent_types() -> Vec<String> {
    RECENT_TYPES
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

fn push_recent_type(node_type: &str) -> Vec<String> {
    let mut next = vec![node_type.to_string()];
    next.extend(
        read_recent_types()
            .into_iter()
            .filter(|t| t != node_type),
    );
    next.truncate(RECENT_LIMIT);
    if let Ok(mut g) = RECENT_TYPES.lock() {
        *g = next.clone();
    }
    next
}

fn node_from_json(n: &serde_json::Value) -> Option<NodeCatalogItem> {
    let category = n
        .get("category")
        .map(json_string)
        .unwrap_or_else(|| "general".into());
    let label = n
        .get("label")
        .or_else(|| n.get("name"))
        .map(json_string)
        .unwrap_or_default();
    let node_type = n.get("type").map(json_string).unwrap_or_default();
    if node_type.is_empty() {
        return None;
    }
    let description = n.get("description").map(json_string).unwrap_or_default();
    Some(NodeCatalogItem {
        node_type,
        label,
        category,
        description,
    })
}

fn node_color(category: &str) -> &'static str {
    match category {
        "trigger" => "#f59e0b",
        "control" => "#8b5cf6",
        "api" => "#3b82f6",
        "data" => "#06b6d4",
        "files" => "#64748b",
        "email" => "#ec4899",
        "microsoft" => "#0078d4",
        _ => "#64748b",
    }
}

fn kind_matches(kind: &str, category: &str) -> bool {
    match kind {
        "triggers" => category == "trigger",
        "actions" => category != "trigger",
        _ => true,
    }
}

#[component]
fn NodeIconGrid(
    items: Vec<NodeCatalogItem>,
    can_add: bool,
    on_add: EventHandler<NodeCatalogItem>,
) -> Element {
    rsx! {
        div { class: "grid grid-cols-3 gap-2 sm:grid-cols-4",
            for item in items {
                {
                    let catalog = item.clone();
                    let color = node_color(&item.category);
                    rsx! {
                        button {
                            key: "{item.node_type}",
                            r#type: "button",
                            disabled: !can_add,
                            title: "{item.label} ({item.node_type})",
                            class: "flex flex-col items-center gap-1.5 rounded-lg border bg-card px-2 py-3 text-center transition-colors hover:border-foreground/20 hover:bg-muted/40 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            onclick: move |_| on_add.call(catalog.clone()),
                            span {
                                class: "flex h-10 w-10 items-center justify-center rounded-xl text-white shadow-sm",
                                style: "background: {color}",
                                if item.category == "trigger" {
                                    Zap { class: Some("h-5 w-5".into()) }
                                } else {
                                    span { class: "text-xs font-bold uppercase",
                                        {item.label.chars().next().unwrap_or('?').to_string()}
                                    }
                                }
                            }
                            span { class: "line-clamp-2 w-full text-[11px] font-medium leading-tight",
                                "{item.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NodesPanel(
    #[props(default)] compact: bool,
    #[props(default)] hide_header: bool,
    #[props(default)] triggers_only: bool,
    #[props(default)] kind_filter: Option<String>,
    #[props(default)] variant: NodesPanelVariant,
    on_add_node: Option<EventHandler<NodeCatalogItem>>,
    #[props(default = true)] can_add: bool,
) -> Element {
    let initial_kind = if triggers_only {
        "triggers".to_string()
    } else {
        kind_filter.clone().unwrap_or_else(|| "all".to_string())
    };
    let kind_filter_prop = kind_filter.clone();

    let mut nodes = use_signal(Vec::<serde_json::Value>::new);
    let mut query = use_signal(String::new);
    let mut category = use_signal(|| "all".to_string());
    let mut kind = use_signal(move || initial_kind);
    let mut recent_types = use_signal(read_recent_types);
    let mut err = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);

    use_effect(move || {
        if triggers_only {
            kind.set("triggers".to_string());
        } else if let Some(k) = kind_filter_prop.clone() {
            kind.set(k);
        }
    });

    use_effect(move || {
        loading.set(true);
        spawn(async move {
            match crate::api::get_json("/api/nodes").await {
                Ok(v) => {
                    nodes.set(
                        v.get("nodes")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let all_items: Vec<NodeCatalogItem> = nodes()
        .iter()
        .filter_map(node_from_json)
        .collect();

    let categories: Vec<String> = {
        let mut set: Vec<String> = all_items.iter().map(|n| n.category.clone()).collect();
        set.sort();
        set.dedup();
        let mut cats = vec!["all".to_string()];
        cats.extend(set);
        cats
    };

    let q = query().trim().to_lowercase();
    let kind_filter = kind();
    let category_filter = category();

    let filtered: Vec<NodeCatalogItem> = all_items
        .iter()
        .filter(|n| {
            if !kind_matches(&kind_filter, &n.category) {
                return false;
            }
            if category_filter != "all" && n.category != category_filter {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            let hay = format!("{} {} {}", n.label, n.node_type, n.category).to_lowercase();
            hay.contains(&q)
        })
        .cloned()
        .collect();

    let mut grouped: Vec<(String, Vec<NodeCatalogItem>)> = Vec::new();
    for item in filtered.clone() {
        if let Some((_, list)) = grouped.iter_mut().find(|(c, _)| c == &item.category) {
            list.push(item);
        } else {
            grouped.push((item.category.clone(), vec![item]));
        }
    }
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    let recent_items: Vec<NodeCatalogItem> = recent_types()
        .into_iter()
        .filter_map(|t| all_items.iter().find(|n| n.node_type == t).cloned())
        .filter(|n| kind_matches(&kind_filter, &n.category))
        .collect();

    let empty = grouped.is_empty();
    let add_handler = on_add_node.clone();
    let show_category_chips = variant == NodesPanelVariant::List;
    let filter_pad = if compact || variant == NodesPanelVariant::Grid {
        "flex flex-col gap-2 border-b p-2"
    } else {
        "flex flex-col gap-2 border-b px-6 py-3"
    };
    let scroll_pad = if compact {
        "flex flex-col gap-4 p-2"
    } else {
        "flex flex-col gap-4 p-6"
    };

    rsx! {
        div { class: "flex h-full flex-col",
            if !hide_header {
                if !compact {
                    div { class: "border-b px-6 py-4",
                        h1 { class: "text-xl font-semibold tracking-tight", "Nodes" }
                        p { class: "text-sm text-muted-foreground",
                            "Browse node types. Open a workflow, then add a node to the canvas."
                        }
                    }
                } else {
                    div { class: "border-b px-3 py-2 text-xs font-medium uppercase text-muted-foreground",
                        "Nodes"
                    }
                }
            }

            div { class: "{filter_pad}",
                input {
                    class: "flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                    placeholder: "Search nodes…",
                    value: "{query()}",
                    oninput: move |e| query.set(e.value()),
                }
                div { class: "flex items-center gap-1 rounded-md bg-muted/50 p-0.5",
                    for (k, label) in [("all", "All"), ("triggers", "Triggers"), ("actions", "Actions")] {
                        button {
                            key: "{k}",
                            r#type: "button",
                            disabled: triggers_only && k != "triggers",
                            class: if kind() == k {
                                "flex flex-1 items-center justify-center gap-1 rounded-md bg-card px-2 py-1.5 text-xs font-medium shadow-sm"
                            } else if triggers_only && k != "triggers" {
                                "flex flex-1 items-center justify-center gap-1 rounded-md px-2 py-1.5 text-xs font-medium text-muted-foreground opacity-40"
                            } else {
                                "flex flex-1 items-center justify-center gap-1 rounded-md px-2 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground"
                            },
                            onclick: move |_| kind.set(k.to_string()),
                            if k == "triggers" {
                                Zap { class: Some("h-3 w-3 text-amber-500".into()) }
                            }
                            "{label}"
                        }
                    }
                }
                if show_category_chips {
                    div { class: "flex flex-wrap gap-1",
                        for c in categories.iter().cloned() {
                            button {
                                key: "{c}",
                                r#type: "button",
                                class: if category() == c {
                                    "rounded-md bg-accent px-2 py-1 text-xs"
                                } else {
                                    "rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-muted"
                                },
                                onclick: move |_| category.set(c.clone()),
                                "{c}"
                            }
                        }
                    }
                }
            }

            if let Some(message) = err() {
                div { class: "px-4 py-2 text-sm text-destructive", "{message}" }
            }

            if loading() {
                PageLoader { label: "Loading nodes…", class: Some("min-h-[8rem]") }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                if variant == NodesPanelVariant::Grid {
                    div { class: "{scroll_pad}",
                        if !recent_items.is_empty() && query().trim().is_empty() {
                            section {
                                div { class: "mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground",
                                    "Recently used"
                                }
                                NodeIconGrid {
                                    items: recent_items.clone(),
                                    can_add,
                                    on_add: {
                                        let handler = add_handler.clone();
                                        move |item: NodeCatalogItem| {
                                            if can_add {
                                                if let Some(h) = handler.as_ref() {
                                                    recent_types.set(push_recent_type(&item.node_type));
                                                    h.call(item);
                                                }
                                            }
                                        }
                                    },
                                }
                            }
                        }
                        for (cat, items) in grouped.clone() {
                            section {
                                key: "{cat}",
                                div { class: "mb-2 flex items-center gap-2",
                                    h2 { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "{cat}" }
                                    Badge { variant: BadgeVariant::Secondary, "{items.len()}" }
                                }
                                NodeIconGrid {
                                    items,
                                    can_add,
                                    on_add: {
                                        let handler = add_handler.clone();
                                        move |item: NodeCatalogItem| {
                                            if can_add {
                                                if let Some(h) = handler.as_ref() {
                                                    recent_types.set(push_recent_type(&item.node_type));
                                                    h.call(item);
                                                }
                                            }
                                        }
                                    },
                                }
                            }
                        }
                        if empty {
                            p { class: "text-sm text-muted-foreground", "No nodes match." }
                        }
                    }
                } else {
                    div { class: "{scroll_pad}",
                        for (cat, items) in grouped.into_iter() {
                            section {
                                key: "{cat}",
                                div { class: "mb-2 flex items-center gap-2",
                                    h2 { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "{cat}" }
                                    Badge { variant: BadgeVariant::Secondary, "{items.len()}" }
                                }
                                div { class: "flex flex-col gap-1",
                                    for item in items.into_iter() {
                                        {
                                            let catalog = item.clone();
                                            let desc = item.description.clone();
                                            let handler = add_handler.clone();
                                            rsx! {
                                                div {
                                                    key: "{item.node_type}",
                                                    class: "flex items-start gap-2 rounded-md border bg-card px-2 py-2",
                                                    div { class: "min-w-0 flex-1",
                                                        div { class: "flex items-center gap-1.5 truncate text-sm font-medium",
                                                            if item.category == "trigger" {
                                                                Zap { class: Some("h-3.5 w-3.5 shrink-0 text-amber-500".into()) }
                                                            }
                                                            "{item.label}"
                                                        }
                                                        div { class: "truncate text-xs text-muted-foreground", "{item.node_type}" }
                                                        if !desc.is_empty() {
                                                            p { class: "mt-1 line-clamp-2 text-[11px] leading-snug text-muted-foreground",
                                                                "{desc}"
                                                            }
                                                        }
                                                    }
                                                    if let Some(handler) = handler {
                                                        Button {
                                                            size: ButtonSize::Sm,
                                                            variant: ButtonVariant::Outline,
                                                            class: Some("shrink-0".into()),
                                                            disabled: !can_add,
                                                            onclick: move |_| {
                                                                if can_add {
                                                                    recent_types.set(push_recent_type(&catalog.node_type));
                                                                    handler.call(catalog.clone());
                                                                }
                                                            },
                                                            "Add"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if empty {
                            p { class: "text-sm text-muted-foreground", "No nodes match." }
                        }
                    }
                }
            }
            }
        }
    }
}