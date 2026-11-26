use dioxus::prelude::*;

use crate::components::icons::{Download, RefreshCw, Trash2};
use crate::components::panels::{json_id, json_string};
use crate::components::ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, ConfirmDeleteDialog,
};

fn action_variant(action: &str) -> BadgeVariant {
    if action == "workflow.deleted" || action == "run.failed" {
        BadgeVariant::Destructive
    } else if action == "run.completed" {
        BadgeVariant::Success
    } else if action.starts_with("run.") {
        BadgeVariant::Outline
    } else {
        BadgeVariant::Default
    }
}

fn format_event_time(at: &str) -> String {
    crate::components::panels::format_local_datetime_short(at)
}

fn details_snippet(details: &serde_json::Value) -> Option<String> {
    if details.is_null() {
        return None;
    }
    if let Some(obj) = details.as_object() {
        if obj.is_empty() {
            return None;
        }
    }
    serde_json::to_string(details).ok()
}

#[component]
pub fn AuditPanel(
    #[props(default)] refresh_key: u32,
    #[props(default = "admin".to_string())] actor_name: String,
) -> Element {
    let mut events = use_signal(Vec::<serde_json::Value>::new);
    let mut err = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    let mut exporting = use_signal(|| false);
    let mut clearing = use_signal(|| false);
    let mut confirm_clear = use_signal(|| false);
    let mut tick = use_signal(|| 0_u32);

    use_effect(move || {
        let _ = refresh_key;
        let _ = tick();
        loading.set(true);
        spawn(async move {
            match crate::api::get_json("/api/audit?limit=80").await {
                Ok(v) => {
                    events.set(
                        v.get("events")
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

    rsx! {
        div { class: "flex h-full flex-col gap-3 p-3",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div { class: "min-w-0",
                    h2 { class: "text-sm font-semibold", "Audit trail" }
                    p { class: "mt-0.5 text-xs text-muted-foreground",
                        "Who created, edited, or deleted a workflow — and when runs started or finished. Signed in as "
                        span { class: "font-medium text-foreground", "{actor_name}" }
                        "."
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: exporting() || (events().is_empty() && !loading()),
                        onclick: move |_| {
                            if exporting() {
                                return;
                            }
                            exporting.set(true);
                            spawn(async move {
                                match crate::api::get_json("/api/audit?limit=5000").await {
                                    Ok(v) => {
                                        let list = v
                                            .get("events")
                                            .and_then(|x| x.as_array())
                                            .cloned()
                                            .unwrap_or_default();
                                        let payload = serde_json::json!({
                                            "exportedAt": chrono::Utc::now().to_rfc3339(),
                                            "count": list.len(),
                                            "events": list,
                                        });
                                        let stamp = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S");
                                        let path = std::env::temp_dir().join(format!("dataflow-audit-{stamp}.json"));
                                        let _ = std::fs::write(
                                            &path,
                                            serde_json::to_string_pretty(&payload).unwrap_or_default(),
                                        );
                                        tracing::info!("Audit export written to {}", path.display());
                                    }
                                    Err(e) => err.set(Some(e)),
                                }
                                exporting.set(false);
                            });
                        },
                        Download { class: Some("h-3.5 w-3.5".into()) }
                        "Export"
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        class: Some("text-destructive hover:bg-destructive/10 hover:text-destructive".into()),
                        disabled: events().is_empty() || clearing(),
                        onclick: move |_| {
                            if events().is_empty() || clearing() {
                                return;
                            }
                            confirm_clear.set(true);
                        },
                        Trash2 { class: Some("h-3.5 w-3.5".into()) }
                        "Clear"
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: loading(),
                        onclick: move |_| tick.set(tick() + 1),
                        RefreshCw {
                            class: Some(if loading() {
                                "h-3.5 w-3.5 animate-spin".into()
                            } else {
                                "h-3.5 w-3.5".into()
                            })
                        }
                        "Refresh"
                    }
                }
            }

            if let Some(message) = err() {
                div { class: "text-sm text-destructive", "{message}" }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "flex flex-col gap-2",
                    for e in events().iter().cloned() {
                        {
                            let id = json_id(&e);
                            let action = e.get("action").map(json_string).unwrap_or_default();
                            let resource = e
                                .get("resourceName")
                                .or_else(|| e.get("resourceId"))
                                .map(json_string)
                                .unwrap_or_default();
                            let resource_type = e.get("resourceType").map(json_string).unwrap_or_default();
                            let actor = e.get("actor").map(json_string).unwrap_or_default();
                            let at_raw = e.get("at").map(json_string).unwrap_or_default();
                            let at = format_event_time(&at_raw);
                            let action_v = action_variant(&action);
                            let details = e
                                .get("details")
                                .and_then(details_snippet);
                            rsx! {
                                div {
                                    key: "{id}",
                                    class: "rounded-md border bg-card p-2 text-sm",
                                    div { class: "flex flex-wrap items-center gap-2",
                                        Badge { variant: action_v, "{action}" }
                                        span { class: "font-medium", "{resource}" }
                                        Badge { variant: BadgeVariant::Outline, "{resource_type}" }
                                    }
                                    div { class: "mt-1 text-xs text-muted-foreground",
                                        span { class: "font-medium text-foreground", "{actor}" }
                                        " · {at}"
                                        if let Some(d) = details {
                                            " · "
                                            span { class: "font-mono text-[10px]", "{d}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if events().is_empty() && err().is_none() && !loading() {
                        div { class: "rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground",
                            "No audit events yet. Create, edit, or run a workflow to populate this trail."
                        }
                    }
                    if loading() && events().is_empty() {
                        div { class: "text-sm text-muted-foreground", "Loading audit events…" }
                    }
                }
            }

            if confirm_clear() {
                ConfirmDeleteDialog {
                    title: "Clear audit trail?".to_string(),
                    description: "This will permanently delete all audit events. This action cannot be undone.".to_string(),
                    confirm_label: "Clear".to_string(),
                    busy: clearing(),
                    on_cancel: move |_| confirm_clear.set(false),
                    on_confirm: move |_| {
                        clearing.set(true);
                        spawn(async move {
                            if crate::api::delete_json("/api/audit").await.is_ok() {
                                events.set(vec![]);
                                confirm_clear.set(false);
                            }
                            clearing.set(false);
                        });
                    },
                }
            }
        }
    }
}