//! Shared API call-access editor (projects, endpoints, path scopes, readonly).

use dioxus::prelude::*;

use crate::components::ui::{Checkbox, Label, Switch, Textarea};

#[derive(Clone, PartialEq, Eq)]
pub struct ProjectOpt {
    pub id: String,
    pub label: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EndpointOpt {
    pub id: String,
    pub project_id: String,
    pub label: String,
}

#[derive(Clone, Default, PartialEq)]
pub struct ApiAccessDraft {
    pub project_ids: Vec<String>,
    pub endpoint_ids: Vec<String>,
    pub scopes_text: String,
    pub readonly: bool,
}

impl ApiAccessDraft {
    pub fn from_json(v: &serde_json::Value) -> Self {
        let project_ids = v
            .get("projectIds")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|i| i.as_str().map(str::trim).filter(|s| !s.is_empty()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let endpoint_ids = v
            .get("endpointIds")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|i| i.as_str().map(str::trim).filter(|s| !s.is_empty()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let scopes_text = v
            .get("scopes")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let method = s.get("method").and_then(|m| m.as_str()).unwrap_or("*");
                        let path = s
                            .get("pathPattern")
                            .and_then(|p| p.as_str())
                            .unwrap_or("*");
                        Some(format!("{method} {path}"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let readonly = v
            .get("readonly")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        Self {
            project_ids,
            endpoint_ids,
            scopes_text,
            readonly,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        if !self.project_ids.is_empty() {
            obj.insert(
                "projectIds".into(),
                serde_json::Value::Array(
                    self.project_ids
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        if !self.endpoint_ids.is_empty() {
            obj.insert(
                "endpointIds".into(),
                serde_json::Value::Array(
                    self.endpoint_ids
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        let mut scopes = Vec::new();
        for line in self.scopes_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(2, char::is_whitespace);
            let method = parts.next().unwrap_or("*").trim();
            let path = parts.next().unwrap_or("*").trim();
            scopes.push(serde_json::json!({
                "method": method,
                "pathPattern": path,
            }));
        }
        if !scopes.is_empty() {
            obj.insert("scopes".into(), serde_json::Value::Array(scopes));
        }
        if self.readonly {
            obj.insert("readonly".into(), serde_json::Value::Bool(true));
        }
        serde_json::Value::Object(obj)
    }

    pub fn is_empty(&self) -> bool {
        self.project_ids.is_empty()
            && self.endpoint_ids.is_empty()
            && self.scopes_text.trim().is_empty()
            && !self.readonly
    }
}

pub fn parse_projects(v: &serde_json::Value) -> Vec<ProjectOpt> {
    let arr = v
        .get("projects")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    arr.into_iter()
        .filter_map(|p| {
            let id = p
                .get("_id")
                .and_then(|x| x.as_str())
                .or_else(|| p.get("id").and_then(|x| x.as_str()))
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                return None;
            }
            let name = p
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let slug = p
                .get("slug")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            Some(ProjectOpt {
                id,
                label: format!("{name} (/apis/{slug})"),
            })
        })
        .collect()
}

fn parse_endpoints(project_id: &str, v: &serde_json::Value) -> Vec<EndpointOpt> {
    let arr = v
        .get("endpoints")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_else(|| v.as_array().cloned().unwrap_or_default());
    arr.into_iter()
        .filter_map(|ep| {
            let id = ep
                .get("id")
                .and_then(|x| x.as_str())
                .or_else(|| ep.get("_id").and_then(|x| x.as_str()))
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                return None;
            }
            let method = ep
                .get("method")
                .and_then(|x| x.as_str())
                .unwrap_or("GET")
                .to_string();
            let path = ep
                .get("path")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let summary = ep
                .get("summary")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let label = if summary.is_empty() {
                format!("{method} /{path}")
            } else {
                format!("{method} /{path} — {summary}")
            };
            Some(EndpointOpt {
                id,
                project_id: project_id.to_string(),
                label,
            })
        })
        .collect()
}

/// Editable call-level access: projects, endpoint picker, path scopes, readonly.
#[component]
pub fn ApiAccessEditor(
    draft: Signal<ApiAccessDraft>,
    projects: Vec<ProjectOpt>,
    #[props(default)] compact: bool,
) -> Element {
    let mut endpoints = use_signal(Vec::<EndpointOpt>::new);
    let mut loading_eps = use_signal(|| false);

    use_effect(move || {
        let ids = draft().project_ids.clone();
        spawn(async move {
            loading_eps.set(true);
            let mut all = Vec::new();
            if ids.is_empty() {
                // Load endpoints for all projects so operators can pick without scoping first.
                if let Ok(v) = crate::api::get_json("/api/api-projects").await {
                    for p in parse_projects(&v) {
                        if let Ok(eps) =
                            crate::api::get_json(&format!("/api/api-projects/{}/endpoints", p.id))
                                .await
                        {
                            all.extend(parse_endpoints(&p.id, &eps));
                        }
                    }
                }
            } else {
                for pid in ids {
                    if let Ok(eps) =
                        crate::api::get_json(&format!("/api/api-projects/{pid}/endpoints")).await
                    {
                        all.extend(parse_endpoints(&pid, &eps));
                    }
                }
            }
            endpoints.set(all);
            loading_eps.set(false);
        });
    });

    let border = if compact {
        "mt-1 max-h-36 space-y-1 overflow-y-auto rounded-md border p-2"
    } else {
        "mt-1 max-h-40 space-y-1 overflow-y-auto rounded-md border p-2"
    };

    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center gap-2",
                Switch {
                    checked: draft().readonly,
                    on_checked_change: move |v| {
                        draft.with_mut(|d| d.readonly = v);
                    },
                }
                Label { class: Some("text-sm".into()), "Read-only (GET/HEAD/OPTIONS only)" }
            }
            div {
                Label { class: Some("text-xs text-muted-foreground".into()), "Project scope" }
                div { class: "{border}",
                    {
                        let all_selected = draft().project_ids.is_empty();
                        rsx! {
                            label { class: "flex cursor-pointer items-center gap-2 rounded-sm px-1 py-1 hover:bg-muted/50",
                                Checkbox {
                                    checked: all_selected,
                                    aria_label: "All API projects".to_string(),
                                    on_checked_change: move |_| {
                                        draft.with_mut(|d| d.project_ids.clear());
                                    },
                                }
                                span { class: "text-sm", "All API projects" }
                            }
                        }
                    }
                    for p in projects.iter().cloned() {
                        {
                            let pid = p.id.clone();
                            let pid_toggle = p.id.clone();
                            let checked = draft().project_ids.iter().any(|id| id == &pid);
                            let label = p.label.clone();
                            rsx! {
                                label { class: "flex cursor-pointer items-center gap-2 rounded-sm px-1 py-1 hover:bg-muted/50",
                                    Checkbox {
                                        checked,
                                        aria_label: label.clone(),
                                        on_checked_change: move |next| {
                                            draft.with_mut(|d| {
                                                if next {
                                                    if !d.project_ids.iter().any(|id| id == &pid_toggle) {
                                                        d.project_ids.push(pid_toggle.clone());
                                                    }
                                                } else {
                                                    d.project_ids.retain(|id| id != &pid_toggle);
                                                    // Drop endpoints that belong to deselected projects when scoped.
                                                }
                                            });
                                        },
                                    }
                                    span { class: "text-sm", "{label}" }
                                }
                            }
                        }
                    }
                }
            }
            div {
                Label { class: Some("text-xs text-muted-foreground".into()),
                    "Endpoints (exact calls)"
                }
                div { class: "{border}",
                    if loading_eps() {
                        p { class: "px-1 py-1 text-xs text-muted-foreground", "Loading endpoints…" }
                    } else if endpoints().is_empty() {
                        p { class: "px-1 py-1 text-xs text-muted-foreground",
                            "No endpoints published yet. Sync apiEndpoint nodes from workflows."
                        }
                    } else {
                        for ep in endpoints() {
                            {
                                let eid = ep.id.clone();
                                let eid_toggle = ep.id.clone();
                                let checked = draft().endpoint_ids.iter().any(|id| id == &eid);
                                let label = ep.label.clone();
                                rsx! {
                                    label { class: "flex cursor-pointer items-center gap-2 rounded-sm px-1 py-1 hover:bg-muted/50",
                                        Checkbox {
                                            checked,
                                            aria_label: label.clone(),
                                            on_checked_change: move |next| {
                                                draft.with_mut(|d| {
                                                    if next {
                                                        if !d.endpoint_ids.iter().any(|id| id == &eid_toggle) {
                                                            d.endpoint_ids.push(eid_toggle.clone());
                                                        }
                                                    } else {
                                                        d.endpoint_ids.retain(|id| id != &eid_toggle);
                                                    }
                                                });
                                            },
                                        }
                                        span { class: "text-sm", "{label}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                Label { class: Some("text-xs text-muted-foreground".into()),
                    "Call scopes (one per line: METHOD /path*)"
                }
                Textarea {
                    class: Some("mt-1 min-h-[4.5rem] text-sm".into()),
                    value: draft().scopes_text.clone(),
                    placeholder: Some("GET /portfolio/*\nPOST /orders".into()),
                    oninput: move |e: FormEvent| {
                        let v = e.value();
                        draft.with_mut(|d| d.scopes_text = v);
                    },
                }
            }
            p { class: "text-xs text-muted-foreground",
                "Empty projects + endpoints + scopes = all calls (within api.invoke). Endpoint ids and path scopes are OR’d when both are set."
            }
        }
    }
}
