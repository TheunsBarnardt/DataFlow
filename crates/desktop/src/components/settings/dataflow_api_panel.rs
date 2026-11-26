use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonVariant, Card, CardContent, CardHeader,
    CardTitle, ConfirmDeleteDialog, Empty, EmptyDescription, Input, Label, Switch,
};

#[derive(Clone)]
struct ApiKeyRow {
    id: String,
    name: String,
    preview: String,
    enabled: bool,
    project: String,
    readonly: bool,
    created: String,
    expires: String,
}

#[derive(Clone)]
struct ProjectOpt {
    id: String,
    label: String,
}

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn format_ts(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return crate::components::panels::format_local_datetime(s);
    }
    if let Some(ms) = v.as_i64() {
        if let Some(dt) = chrono::DateTime::from_timestamp_millis(ms) {
            return dt
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
        }
        return ms.to_string();
    }
    "—".into()
}

fn parse_keys(v: &serde_json::Value) -> Vec<ApiKeyRow> {
    let arr = v
        .get("apiKeys")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    arr.into_iter()
        .filter_map(|row| {
            let id = json_str(&row, "id");
            let id = if id.is_empty() {
                json_str(&row, "_id")
            } else {
                id
            };
            if id.is_empty() {
                return None;
            }
            let prefix = json_str(&row, "prefix");
            let start = json_str(&row, "start");
            let preview = if start.is_empty() {
                format!("{}…", if prefix.is_empty() { "bf_" } else { &prefix })
            } else {
                format!(
                    "{}{}…",
                    if prefix.is_empty() { "bf_" } else { &prefix },
                    start
                )
            };
            let meta = row.get("metadata").cloned().unwrap_or(serde_json::Value::Null);
            let project = meta
                .get("projectId")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let readonly = meta
                .get("readonly")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            Some(ApiKeyRow {
                id,
                name: {
                    let n = json_str(&row, "name");
                    if n.is_empty() {
                        "Untitled key".into()
                    } else {
                        n
                    }
                },
                preview,
                enabled: row
                    .get("enabled")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(true),
                project,
                readonly,
                created: row
                    .get("createdAt")
                    .map(format_ts)
                    .unwrap_or_else(|| "—".into()),
                expires: row
                    .get("expiresAt")
                    .map(|x| {
                        if x.is_null() {
                            "never".into()
                        } else {
                            format_ts(x)
                        }
                    })
                    .unwrap_or_else(|| "never".into()),
            })
        })
        .collect()
}

fn parse_projects(v: &serde_json::Value) -> Vec<ProjectOpt> {
    let arr = v
        .get("projects")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    arr.into_iter()
        .filter_map(|p| {
            let id = json_str(&p, "_id");
            let id = if id.is_empty() {
                json_str(&p, "id")
            } else {
                id
            };
            if id.is_empty() {
                return None;
            }
            let name = json_str(&p, "name");
            let slug = json_str(&p, "slug");
            Some(ProjectOpt {
                id,
                label: format!("{name} (/apis/{slug})"),
            })
        })
        .collect()
}

#[component]
pub fn DataFlowApiPanel() -> Element {
    let mut keys = use_signal(Vec::<ApiKeyRow>::new);
    let mut projects = use_signal(Vec::<ProjectOpt>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut name = use_signal(String::new);
    let mut expires_in = use_signal(String::new);
    let mut project_id = use_signal(String::new);
    let mut readonly = use_signal(|| false);
    let mut creating = use_signal(|| false);
    let mut created_secret = use_signal(|| None::<String>);
    let mut busy_id = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let keys_res = crate::api::get_json("/api/settings/api-keys").await;
            let projects_res = crate::api::get_json("/api/api-projects").await;
            match keys_res {
                Ok(v) => {
                    keys.set(parse_keys(&v));
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
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

    let create = move |_| {
        let trimmed = name().trim().to_string();
        if trimmed.is_empty() {
            err.set(Some("Enter a name for the API key".into()));
            return;
        }
        creating.set(true);
        let exp = expires_in();
        let proj = project_id();
        let ro = readonly();
        spawn(async move {
            let mut body = serde_json::json!({ "name": trimmed });
            if let Ok(secs) = exp.parse::<i64>() {
                if secs > 0 {
                    body["expiresIn"] = serde_json::json!(secs);
                }
            }
            let mut meta = serde_json::Map::new();
            if !proj.is_empty() {
                meta.insert("projectId".into(), serde_json::Value::String(proj));
            }
            if ro {
                meta.insert("readonly".into(), serde_json::Value::Bool(true));
            }
            if !meta.is_empty() {
                body["metadata"] = serde_json::Value::Object(meta);
            }
            match crate::api::post_json("/api/settings/api-keys", body).await {
                Ok(v) => {
                    let secret = v
                        .pointer("/key/key")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                    if let Some(secret) = secret {
                        created_secret.set(Some(secret));
                    }
                    name.set(String::new());
                    expires_in.set(String::new());
                    project_id.set(String::new());
                    readonly.set(false);
                    err.set(None);
                    if let Ok(list) = crate::api::get_json("/api/settings/api-keys").await {
                        keys.set(parse_keys(&list));
                    }
                }
                Err(e) => err.set(Some(e)),
            }
            creating.set(false);
        });
    };

    rsx! {
        div { class: "flex h-full flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25] tracking-normal",
                    "DataFlow API"
                }
                p { class: "mt-3 text-sm leading-[1.35] text-muted-foreground",
                    "Create instance API keys for public API project endpoints. Send the key in the "
                    code { class: "text-foreground", "X-Api-Key" }
                    " header to "
                    code { class: "text-foreground", "/apis/{{slug}}/…" }
                    " when the endpoint auth mode is API Key."
                }
            }

            div { class: "space-y-6 p-6",
                Card {
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()), "Public API access" }
                    }
                    CardContent { class: Some("px-4 pb-4 pt-2".into()),
                        p { class: "text-sm text-muted-foreground",
                            "Each API project is served at "
                            code { class: "text-foreground", "/apis/{{slug}}" }
                            " with OpenAPI and Swagger UI under that path. Keys are stored hashed in MongoDB ("
                            code { class: "text-foreground", "bf_" }
                            " prefix). Studio "
                            code { class: "text-foreground", "/api/*" }
                            " routes still use your signed-in session."
                        }
                    }
                }

                if let Some(message) = err() {
                    Alert {
                        variant: AlertVariant::Destructive,
                        AlertDescription {
                            class: Some("text-destructive".into()),
                            "{message}"
                        }
                    }
                }

                if let Some(secret) = created_secret() {
                    Alert {
                        variant: AlertVariant::Success,
                        AlertDescription {
                            div { class: "space-y-2 text-foreground",
                                p { class: "text-sm font-semibold", "Copy your new API key" }
                                p { class: "text-sm text-muted-foreground",
                                    "This secret is shown once. Store it securely."
                                }
                                code { class: "block break-all rounded-md border bg-background px-3 py-2 text-sm",
                                    "{secret}"
                                }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| created_secret.set(None),
                                    "Dismiss"
                                }
                            }
                        }
                    }
                }

                Card {
                    CardContent { class: Some("p-4".into()),
                    h2 { class: "text-sm font-semibold", "Create API key" }
                    div { class: "mt-3 grid gap-3 md:grid-cols-2 xl:grid-cols-4",
                        div {
                            Label { class: Some("text-xs text-muted-foreground".into()),
                                "Name"
                            }
                            Input {
                                class: Some("mt-1 h-9".into()),
                                value: name(),
                                placeholder: Some("CI integration".into()),
                                oninput: move |e: FormEvent| name.set(e.value()),
                            }
                        }
                        div {
                            Label { class: Some("text-xs text-muted-foreground".into()),
                                "Expires"
                            }
                            select {
                                class: "mt-1 flex h-9 w-full rounded-md border bg-background px-3 text-sm",
                                value: expires_in(),
                                onchange: move |e: FormEvent| expires_in.set(e.value()),
                                option { value: "", "No expiry" }
                                option { value: "604800", "7 days" }
                                option { value: "2592000", "30 days" }
                                option { value: "7776000", "90 days" }
                                option { value: "31536000", "1 year" }
                            }
                        }
                        div {
                            Label { class: Some("text-xs text-muted-foreground".into()),
                                "Project scope"
                            }
                            select {
                                class: "mt-1 flex h-9 w-full rounded-md border bg-background px-3 text-sm",
                                value: project_id(),
                                onchange: move |e: FormEvent| project_id.set(e.value()),
                                option { value: "", "All API projects" }
                                for p in projects() {
                                    option { value: "{p.id}", "{p.label}" }
                                }
                            }
                        }
                        div { class: "flex items-end justify-between gap-3",
                            div { class: "flex items-center gap-2 pb-1.5",
                                Switch {
                                    checked: readonly(),
                                    on_checked_change: move |v| readonly.set(v),
                                }
                                Label { class: Some("text-sm".into()), "Read-only" }
                            }
                            Button {
                                disabled: creating(),
                                onclick: create,
                                "Create"
                            }
                        }
                    }
                    p { class: "mt-3 text-xs text-muted-foreground",
                        "Read-only keys may only call GET, HEAD, and OPTIONS. Project scope limits the key to one API project."
                    }
                    }
                }

                Card { class: Some("overflow-hidden p-0 gap-0".into()),
                    div { class: "flex items-center justify-between border-b bg-muted/40 px-4 py-3",
                        h2 { class: "text-sm font-semibold", "API keys" }
                        span { class: "text-xs text-muted-foreground",
                            if loading() {
                                "Loading…"
                            } else {
                                "{keys().len()} keys"
                            }
                        }
                    }
                    if keys().is_empty() && !loading() {
                        Empty { class: Some("border-0 py-8".into()),
                            EmptyDescription {
                                "No API keys yet. Create one to call public API project endpoints."
                            }
                        }
                    } else {
                        div { class: "divide-y",
                            for row in keys() {
                                {
                                    let id = row.id.clone();
                                    let id_toggle = row.id.clone();
                                    let name = row.name.clone();
                                    let enabled = row.enabled;
                                    let busy = busy_id().as_deref() == Some(row.id.as_str());
                                    let scope_label = if row.project.is_empty() {
                                        "All projects".to_string()
                                    } else {
                                        row.project.clone()
                                    };
                                    let status_label = if enabled { "Enabled" } else { "Disabled" };
                                    let toggle_label = if enabled { "Disable" } else { "Enable" };
                                    rsx! {
                                        div { class: "flex flex-wrap items-center gap-3 px-4 py-3",
                                            div { class: "min-w-[12rem] flex-1",
                                                div { class: "text-sm font-medium", "{row.name}" }
                                                code { class: "text-[11px] text-muted-foreground", "{row.preview}" }
                                            }
                                            div { class: "text-xs text-muted-foreground",
                                                if row.readonly {
                                                    "Read-only · "
                                                }
                                                "{scope_label} · {status_label}"
                                            }
                                            div { class: "text-xs text-muted-foreground",
                                                "Created {row.created} · Expires {row.expires}"
                                            }
                                            div { class: "ml-auto flex items-center gap-2",
                                                Button {
                                                    variant: ButtonVariant::Outline,
                                                    disabled: busy,
                                                    onclick: move |_| {
                                                        busy_id.set(Some(id_toggle.clone()));
                                                        let next = !enabled;
                                                        let key_id = id_toggle.clone();
                                                        spawn(async move {
                                                            match crate::api::patch_json(
                                                                &format!("/api/settings/api-keys/{key_id}"),
                                                                serde_json::json!({ "enabled": next }),
                                                            ).await {
                                                                Ok(_) => {
                                                                    if let Ok(list) = crate::api::get_json("/api/settings/api-keys").await {
                                                                        keys.set(parse_keys(&list));
                                                                    }
                                                                    err.set(None);
                                                                }
                                                                Err(e) => err.set(Some(e)),
                                                            }
                                                            busy_id.set(None);
                                                        });
                                                    },
                                                    "{toggle_label}"
                                                }
                                                Button {
                                                    variant: ButtonVariant::Ghost,
                                                    disabled: busy,
                                                    onclick: move |_| {
                                                        confirm_delete.set(Some((id.clone(), name.clone())));
                                                    },
                                                    "Delete"
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

            if let Some((del_id, del_name)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let description = format!(
                        "This will permanently revoke API key \"{del_name}\". This action cannot be undone."
                    );
                    let busy = busy_id().as_deref() == Some(del_id.as_str());
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete API key?".to_string(),
                            description,
                            busy,
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                busy_id.set(Some(del_id_confirm.clone()));
                                let key_id = del_id_confirm.clone();
                                spawn(async move {
                                    match crate::api::delete_json(
                                        &format!("/api/settings/api-keys/{key_id}"),
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            if let Ok(list) =
                                                crate::api::get_json("/api/settings/api-keys").await
                                            {
                                                keys.set(parse_keys(&list));
                                            }
                                            err.set(None);
                                            confirm_delete.set(None);
                                        }
                                        Err(e) => err.set(Some(e)),
                                    }
                                    busy_id.set(None);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}
