use dioxus::prelude::*;

use super::api_access_editor::{
    parse_projects, ApiAccessDraft, ApiAccessEditor, ProjectOpt,
};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonVariant, Card, CardContent, CardHeader,
    CardTitle, Checkbox, Empty, EmptyDescription, Input, Label,
};

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn perm_ids(v: &serde_json::Value) -> Vec<String> {
    v.get("permissions")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn role_has_api_invoke(perms: &[String]) -> bool {
    perms.iter().any(|p| p == "api.invoke" || p == "*")
}

#[component]
pub fn RolesPanel() -> Element {
    let mut roles = use_signal(Vec::<serde_json::Value>::new);
    let mut catalog = use_signal(Vec::<serde_json::Value>::new);
    let mut projects = use_signal(Vec::<ProjectOpt>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut new_id = use_signal(String::new);
    let mut new_name = use_signal(String::new);
    let mut new_desc = use_signal(String::new);
    let mut selected_perms = use_signal(Vec::<String>::new);
    let mut access = use_signal(ApiAccessDraft::default);
    let mut creating = use_signal(|| false);
    let mut editing_id = use_signal(|| None::<String>);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let roles_res = crate::api::get_json("/api/roles").await;
            let perms_res = crate::api::get_json("/api/permissions").await;
            let projects_res = crate::api::get_json("/api/api-projects").await;
            match roles_res {
                Ok(v) => {
                    roles.set(
                        v.get("roles")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            if let Ok(v) = perms_res {
                catalog.set(
                    v.get("permissions")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Ok(v) = projects_res {
                projects.set(parse_projects(&v));
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let save = move |_| {
        let id = new_id().trim().to_string();
        let name = new_name().trim().to_string();
        if id.is_empty() || name.is_empty() {
            err.set(Some("Role id and name required".into()));
            return;
        }
        creating.set(true);
        let desc = new_desc();
        let perms = selected_perms();
        let draft = access();
        let edit = editing_id();
        spawn(async move {
            let api_access = if role_has_api_invoke(&perms) {
                Some(draft.to_json())
            } else {
                Some(serde_json::json!({}))
            };
            let result = if let Some(rid) = edit {
                crate::api::patch_json(
                    &format!("/api/roles/{rid}"),
                    serde_json::json!({
                        "name": name,
                        "description": desc,
                        "permissions": perms,
                        "apiAccess": api_access,
                    }),
                )
                .await
            } else {
                crate::api::post_json(
                    "/api/roles",
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "description": desc,
                        "permissions": perms,
                        "apiAccess": api_access,
                    }),
                )
                .await
            };
            match result {
                Ok(_) => {
                    new_id.set(String::new());
                    new_name.set(String::new());
                    new_desc.set(String::new());
                    selected_perms.set(Vec::new());
                    access.set(ApiAccessDraft::default());
                    editing_id.set(None);
                    if let Ok(v) = crate::api::get_json("/api/roles").await {
                        roles.set(
                            v.get("roles")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            creating.set(false);
        });
    };

    let show_access = role_has_api_invoke(&selected_perms());
    let is_editing = editing_id().is_some();

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25] tracking-normal", "Roles" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Define roles, permissions, and default API call access. Users inherit role access unless they override it."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription { class: Some("text-destructive".into()), "{message}" }
                }
            }

            div { class: "grid min-h-0 flex-1 gap-6 p-6 lg:grid-cols-[minmax(320px,420px)_1fr]",
                Card { class: Some("self-start".into()),
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()),
                            if is_editing { "Edit role" } else { "Create role" }
                        }
                    }
                    CardContent { class: Some("space-y-3 px-4 pb-4".into()),
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Id" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_id(),
                                placeholder: Some("portfolio_viewer".into()),
                                disabled: is_editing,
                                oninput: move |e: FormEvent| new_id.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_name(),
                                placeholder: Some("Portfolio viewer".into()),
                                oninput: move |e: FormEvent| new_name.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Description" }
                            Input {
                                class: Some("h-9".into()),
                                value: new_desc(),
                                oninput: move |e: FormEvent| new_desc.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Permissions" }
                            div { class: "max-h-48 space-y-0.5 overflow-y-auto rounded-md border p-1.5",
                                for p in catalog() {
                                    {
                                        let pid = json_str(&p, "id");
                                        let label = json_str(&p, "label");
                                        let checked = selected_perms().iter().any(|x| x == &pid);
                                        let pid_t = pid.clone();
                                        rsx! {
                                            label { class: "flex cursor-pointer items-start gap-2 rounded-sm px-1.5 py-1.5 text-sm hover:bg-muted/50",
                                                Checkbox {
                                                    class: Some("mt-0.5".into()),
                                                    checked,
                                                    aria_label: label.clone(),
                                                    on_checked_change: move |next| {
                                                        selected_perms.with_mut(|ids| {
                                                            if next {
                                                                if !ids.iter().any(|x| x == &pid_t) {
                                                                    ids.push(pid_t.clone());
                                                                }
                                                            } else {
                                                                ids.retain(|x| x != &pid_t);
                                                            }
                                                        });
                                                    },
                                                }
                                                span { class: "min-w-0 leading-snug",
                                                    span { class: "block", "{label}" }
                                                    code { class: "text-[11px] text-muted-foreground", "{pid}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if show_access {
                            div { class: "rounded-md border p-3",
                                Label { class: Some("mb-2 block text-xs font-medium".into()),
                                    "Default API call access"
                                }
                                ApiAccessEditor {
                                    draft: access,
                                    projects: projects(),
                                    compact: true,
                                }
                            }
                        }
                        div { class: "flex items-center gap-2",
                            Button { disabled: creating(), onclick: save,
                                if is_editing { "Save changes" } else { "Create role" }
                            }
                            if is_editing {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| {
                                        editing_id.set(None);
                                        new_id.set(String::new());
                                        new_name.set(String::new());
                                        new_desc.set(String::new());
                                        selected_perms.set(Vec::new());
                                        access.set(ApiAccessDraft::default());
                                    },
                                    "Cancel"
                                }
                            }
                        }
                    }
                }

                Card { class: Some("flex min-h-0 min-w-0 flex-col overflow-hidden p-0 gap-0".into()),
                    div { class: "shrink-0 border-b bg-muted/40 px-4 py-3",
                        h2 { class: "text-sm font-semibold",
                            if loading() {
                                "Loading…"
                            } else {
                                "{roles().len()} roles"
                            }
                        }
                    }
                    if loading() {
                        CardContent { class: Some("p-4 text-sm text-muted-foreground".into()), "Loading…" }
                    } else if roles().is_empty() {
                        Empty { class: Some("border-0 py-10".into()),
                            EmptyDescription { "No roles" }
                        }
                    } else {
                        div { class: "min-h-0 flex-1 divide-y overflow-y-auto",
                            for role in roles() {
                                {
                                    let name = json_str(&role, "name");
                                    let id = json_str(&role, "id");
                                    let desc = json_str(&role, "description");
                                    let builtin = role.get("builtin").and_then(|x| x.as_bool()).unwrap_or(false);
                                    let perms = perm_ids(&role);
                                    let api_access = role.get("apiAccess").cloned().unwrap_or(serde_json::json!({}));
                                    let draft = ApiAccessDraft::from_json(&api_access);
                                    let access_summary = if !role_has_api_invoke(&perms) {
                                        "No api.invoke".to_string()
                                    } else if draft.is_empty() {
                                        "All API calls".to_string()
                                    } else {
                                        let mut parts = Vec::new();
                                        if !draft.project_ids.is_empty() {
                                            parts.push(format!("{} projects", draft.project_ids.len()));
                                        }
                                        if !draft.endpoint_ids.is_empty() {
                                            parts.push(format!("{} endpoints", draft.endpoint_ids.len()));
                                        }
                                        if !draft.scopes_text.trim().is_empty() {
                                            parts.push("path scopes".into());
                                        }
                                        if draft.readonly {
                                            parts.push("readonly".into());
                                        }
                                        parts.join(" · ")
                                    };
                                    let id_edit = id.clone();
                                    let name_edit = name.clone();
                                    let desc_edit = desc.clone();
                                    let perms_edit = perms.clone();
                                    let access_edit = draft.clone();
                                    rsx! {
                                        div { class: "space-y-2 px-4 py-3",
                                            div { class: "flex flex-wrap items-start justify-between gap-2",
                                                div { class: "min-w-0",
                                                    div { class: "flex flex-wrap items-center gap-2",
                                                        span { class: "text-sm font-medium", "{name}" }
                                                        if builtin {
                                                            span { class: "rounded border px-1.5 py-0.5 text-[11px] text-muted-foreground",
                                                                "Built-in"
                                                            }
                                                        }
                                                    }
                                                    code { class: "text-[11px] text-muted-foreground", "{id}" }
                                                }
                                                Button {
                                                    variant: ButtonVariant::Outline,
                                                    onclick: move |_| {
                                                        editing_id.set(Some(id_edit.clone()));
                                                        new_id.set(id_edit.clone());
                                                        new_name.set(name_edit.clone());
                                                        new_desc.set(desc_edit.clone());
                                                        selected_perms.set(perms_edit.clone());
                                                        access.set(access_edit.clone());
                                                    },
                                                    "Edit"
                                                }
                                            }
                                            if !desc.is_empty() {
                                                p { class: "text-sm text-muted-foreground", "{desc}" }
                                            }
                                            if !perms.is_empty() {
                                                div { class: "flex flex-wrap gap-1",
                                                    for p in perms {
                                                        span { class: "rounded border px-1.5 py-0.5 text-[11px]", "{p}" }
                                                    }
                                                }
                                            }
                                            p { class: "text-xs text-muted-foreground", "API access: {access_summary}" }
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
