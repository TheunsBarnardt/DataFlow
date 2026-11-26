use dioxus::prelude::*;

use crate::components::icons::{Folder, Play};
use crate::components::panels::{format_local_datetime, json_bool, json_id, json_string};
use crate::components::layout::NavSection;
use crate::components::ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardAction, CardContent,
    CardDescription, CardHeader, CardSize, CardTitle, Empty, EmptyDescription, EmptyHeader,
    EmptyMedia, EmptyMediaVariant, EmptyTitle, Skeleton, Switch,
};

#[component]
pub fn DashboardPanel(
    on_open_workflow: EventHandler<String>,
    on_navigate: EventHandler<NavSection>,
    on_toggle_enabled: EventHandler<(String, bool)>,
    #[props(default)] refresh_key: u32,
) -> Element {
    let mut workflows = use_signal(Vec::<serde_json::Value>::new);
    let mut runs = use_signal(Vec::<serde_json::Value>::new);
    let mut variable_count = use_signal(|| 0usize);
    let mut node_count = use_signal(|| 0usize);
    let mut loading = use_signal(|| true);

    use_effect(move || {
        let _ = refresh_key;
        loading.set(true);
        spawn(async move {
            if let Ok(v) = crate::api::get_json("/api/workflows").await {
                workflows.set(
                    v.get("workflows")
                        .and_then(|w| w.as_array())
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Ok(r) = crate::api::get_json("/api/runs?limit=8").await {
                runs.set(
                    r.get("runs")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Ok(v) = crate::api::get_json("/api/variables").await {
                variable_count.set(
                    v.get("variables")
                        .and_then(|x| x.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0),
                );
            }
            if let Ok(n) = crate::api::get_json("/api/nodes").await {
                node_count.set(
                    n.get("nodes")
                        .and_then(|x| x.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0),
                );
            }
            loading.set(false);
        });
    });

    if loading() {
        return rsx! {
            div { class: "h-full overflow-y-auto",
                div { class: "flex flex-col gap-6 p-6",
                    div { class: "space-y-2",
                        Skeleton { class: Some("h-7 w-40".into()) }
                        Skeleton { class: Some("h-4 w-72".into()) }
                    }
                    div { class: "grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
                        for _ in 0..4 {
                            Skeleton { class: Some("h-24 w-full rounded-xl".into()) }
                        }
                    }
                    div { class: "grid gap-4 lg:grid-cols-2",
                        Skeleton { class: Some("h-64 w-full rounded-xl".into()) }
                        Skeleton { class: Some("h-64 w-full rounded-xl".into()) }
                    }
                }
            }
        };
    }

    let wf = workflows();
    let enabled = wf.iter().filter(|w| json_bool(w, "enabled", true)).count();
    let failed = runs()
        .iter()
        .filter(|r| r.get("status").map(json_string).as_deref() == Some("failed"))
        .count();

    rsx! {
        div { class: "h-full overflow-y-auto",
            div { class: "flex flex-col gap-6 p-6",
                div {
                    h1 { class: "text-xl font-semibold tracking-tight", "Dashboard" }
                    p { class: "text-sm text-muted-foreground",
                        "Overview of workflows, runs, variables, and available nodes."
                    }
                }

                div { class: "grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
                    StatCard {
                        label: "Workflows",
                        value: wf.len(),
                        onclick: move |_| on_navigate.call(NavSection::Workflows),
                    }
                    StatCard {
                        label: "Enabled",
                        value: enabled,
                        onclick: move |_| on_navigate.call(NavSection::Workflows),
                    }
                    StatCard {
                        label: "Variables",
                        value: variable_count(),
                        onclick: move |_| on_navigate.call(NavSection::Variables),
                    }
                    StatCard {
                        label: "Node types",
                        value: node_count(),
                        onclick: move |_| on_navigate.call(NavSection::Nodes),
                    }
                }

                div { class: "grid gap-4 lg:grid-cols-2",
                    Card { size: CardSize::Sm,
                        CardHeader { class: Some("border-b".into()),
                            CardTitle { class: Some("text-sm".into()), "Recent workflows" }
                            CardDescription { "Jump back into a project" }
                            CardAction {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    onclick: move |_| on_navigate.call(NavSection::Workflows),
                                    "Open"
                                }
                            }
                        }
                        CardContent { class: Some("pt-1".into()),
                            div { class: "flex flex-col gap-1",
                                for w in wf.iter().take(6) {
                                    {
                                        let id = json_id(w);
                                        let name = w.get("name").map(json_string).unwrap_or_else(|| "Untitled".into());
                                        let enabled = json_bool(w, "enabled", true);
                                        let id_open = id.clone();
                                        let id_toggle = id.clone();
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                class: "flex items-center justify-between gap-2 rounded-md px-2 py-2 text-sm hover:bg-muted",
                                                button {
                                                    r#type: "button",
                                                    class: "min-w-0 flex-1 truncate text-left font-medium",
                                                    onclick: move |_| on_open_workflow.call(id_open.clone()),
                                                    "{name}"
                                                }
                                                Switch {
                                                    checked: enabled,
                                                    size: Some("sm".into()),
                                                    on_checked_change: move |checked| {
                                                        on_toggle_enabled.call((id_toggle.clone(), checked));
                                                    },
                                                }
                                            }
                                        }
                                    }
                                }
                                if wf.is_empty() {
                                    Empty { class: Some("border-0 p-6".into()),
                                        EmptyHeader {
                                            EmptyMedia { variant: EmptyMediaVariant::Icon,
                                                Folder { class: Some("size-5".into()) }
                                            }
                                            EmptyTitle { class: Some("text-sm".into()), "No workflows yet" }
                                            EmptyDescription {
                                                "Create one from Projects or the + button."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Card { size: CardSize::Sm,
                        CardHeader { class: Some("border-b".into()),
                            CardTitle { class: Some("text-sm".into()), "Recent runs" }
                            CardDescription { "Latest execution activity" }
                            CardAction {
                                div { class: "flex items-center gap-2",
                                    if failed > 0 {
                                        Badge { variant: BadgeVariant::Destructive, "{failed} failed" }
                                    }
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        size: ButtonSize::Sm,
                                        onclick: move |_| on_navigate.call(NavSection::Runs),
                                        "All runs"
                                    }
                                }
                            }
                        }
                        CardContent { class: Some("pt-1".into()),
                            div { class: "flex flex-col gap-1",
                                for r in runs().iter() {
                                    {
                                        let id = json_id(r);
                                        let name = r.get("workflowName").map(json_string).unwrap_or_default();
                                        let status = r.get("status").map(json_string).unwrap_or_default();
                                        let trigger = r.get("trigger").map(json_string).unwrap_or_default();
                                        let started = format_local_datetime(
                                            &r.get("startedAt").map(json_string).unwrap_or_default(),
                                        );
                                        let error = r.get("error").map(json_string).filter(|s| !s.is_empty());
                                        let variant = match status.as_str() {
                                            "failed" => BadgeVariant::Destructive,
                                            "completed" | "succeeded" => BadgeVariant::Success,
                                            _ => BadgeVariant::Secondary,
                                        };
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                class: "flex flex-col gap-1 rounded-md px-2 py-2 text-sm hover:bg-muted/60",
                                                div { class: "flex items-center justify-between gap-2",
                                                    div { class: "min-w-0",
                                                        div { class: "truncate font-medium", "{name}" }
                                                        div { class: "text-xs text-muted-foreground",
                                                            "{trigger} · {started}"
                                                        }
                                                    }
                                                    Badge { variant, "{status}" }
                                                }
                                                if let Some(err) = error {
                                                    div { class: "text-xs leading-snug text-destructive line-clamp-2",
                                                        "{err}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if runs().is_empty() {
                                    Empty { class: Some("border-0 p-6".into()),
                                        EmptyHeader {
                                            EmptyMedia { variant: EmptyMediaVariant::Icon,
                                                Play { class: Some("size-5".into()) }
                                            }
                                            EmptyTitle { class: Some("text-sm".into()), "No runs yet" }
                                            EmptyDescription { "Execute a workflow to see activity here." }
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

#[component]
fn StatCard(label: &'static str, value: usize, onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "rounded-xl border bg-card p-4 text-left shadow-sm transition-all hover:bg-accent/60 hover:shadow-md active:scale-[0.99]",
            onclick: move |_| onclick.call(()),
            div { class: "text-xs font-medium uppercase tracking-wide text-muted-foreground", "{label}" }
            div { class: "mt-2 text-2xl font-semibold tabular-nums tracking-tight", "{value}" }
        }
    }
}
