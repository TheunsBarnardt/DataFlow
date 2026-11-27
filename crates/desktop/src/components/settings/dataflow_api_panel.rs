use dioxus::prelude::*;

use super::api_access_editor::{
    parse_projects, ApiAccessDraft, ApiAccessEditor, ProjectOpt,
};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonVariant, Card, CardContent, CardHeader,
    CardTitle, ConfirmDeleteDialog, Empty, EmptyDescription, Input, Label, Select, SelectOption,
};

#[derive(Clone)]
struct ApiKeyRow {
    id: String,
    name: String,
    preview: String,
    enabled: bool,
    project_ids: Vec<String>,
    endpoint_ids: Vec<String>,
    scopes_text: String,
    readonly: bool,
    created: String,
    expires: String,
    metadata: serde_json::Value,
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
            let meta = row
                .get("metadata")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let draft = ApiAccessDraft::from_json(&meta);
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
                project_ids: draft.project_ids.clone(),
                endpoint_ids: draft.endpoint_ids.clone(),
                scopes_text: draft.scopes_text.clone(),
                readonly: draft.readonly,
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
                metadata: meta,
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
    let mut company_id = use_signal(String::new);
    let mut companies = use_signal(Vec::<ProjectOpt>::new);
    let mut access = use_signal(ApiAccessDraft::default);
    let mut creating = use_signal(|| false);
    let mut created_secret = use_signal(|| None::<String>);
    let mut busy_id = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);
    let mut editing_id = use_signal(|| None::<String>);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let keys_res = crate::api::get_json("/api/settings/api-keys").await;
            let projects_res = crate::api::get_json("/api/api-projects").await;
            let companies_res = crate::api::get_json("/api/companies").await;
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
            if let Ok(v) = companies_res {
                let arr = v
                    .get("companies")
                    .and_then(|x| x.as_array())
                    .cloned()
                    .unwrap_or_default();
                companies.set(
                    arr.into_iter()
                        .filter_map(|c| {
                            let id = json_str(&c, "id");
                            if id.is_empty() {
                                return None;
                            }
                            Some(ProjectOpt {
                                id,
                                label: json_str(&c, "name"),
                            })
                        })
                        .collect(),
                );
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let save = move |_| {
        let trimmed = name().trim().to_string();
        if trimmed.is_empty() {
            err.set(Some("Enter a name for the API key".into()));
            return;
        }
        creating.set(true);
        let exp = expires_in();
        let company = company_id();
        let draft = access();
        let edit_id = editing_id();
        spawn(async move {
            let mut meta = draft.to_json();
            if let Some(obj) = meta.as_object_mut() {
                if !company.trim().is_empty() {
                    obj.insert(
                        "companyId".into(),
                        serde_json::Value::String(company.trim().to_string()),
                    );
                }
            }
            if let Some(key_id) = edit_id {
                let body = serde_json::json!({
                    "name": trimmed,
                    "metadata": meta,
                });
                match crate::api::patch_json(&format!("/api/settings/api-keys/{key_id}"), body).await
                {
                    Ok(_) => {
                        name.set(String::new());
                        expires_in.set(String::new());
                        company_id.set(String::new());
                        access.set(ApiAccessDraft::default());
                        editing_id.set(None);
                        err.set(None);
                        if let Ok(list) = crate::api::get_json("/api/settings/api-keys").await {
                            keys.set(parse_keys(&list));
                        }
                    }
                    Err(e) => err.set(Some(e)),
                }
            } else {
                let mut body = serde_json::json!({ "name": trimmed, "metadata": meta });
                if let Ok(secs) = exp.parse::<i64>() {
                    if secs > 0 {
                        body["expiresIn"] = serde_json::json!(secs);
                    }
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
                        company_id.set(String::new());
                        access.set(ApiAccessDraft::default());
                        err.set(None);
                        if let Ok(list) = crate::api::get_json("/api/settings/api-keys").await {
                            keys.set(parse_keys(&list));
                        }
                    }
                    Err(e) => err.set(Some(e)),
                }
            }
            creating.set(false);
        });
    };

    let is_editing = editing_id().is_some();

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25] tracking-normal",
                    "API Integration"
                }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Issue scoped API keys for external callers. Keys use X-Api-Key; users authenticate with JWT and their own call-level access."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription { class: Some("text-destructive".into()), "{message}" }
                }
            }

            if let Some(secret) = created_secret() {
                Alert {
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription {
                        "Copy this key now — it will not be shown again: "
                        code { class: "break-all text-xs", "{secret}" }
                        Button {
                            class: Some("ml-2".into()),
                            variant: ButtonVariant::Outline,
                            onclick: move |_| created_secret.set(None),
                            "Dismiss"
                        }
                    }
                }
            }

            div { class: "grid min-h-0 flex-1 gap-6 p-6 lg:grid-cols-[minmax(320px,420px)_1fr]",
                Card { class: Some("self-start".into()),
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()),
                            if is_editing { "Edit API key" } else { "Create API key" }
                        }
                    }
                    CardContent { class: Some("space-y-3 px-4 pb-4".into()),
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                            Input {
                                class: Some("h-9".into()),
                                value: name(),
                                placeholder: Some("Mobile app".into()),
                                oninput: move |e: FormEvent| name.set(e.value()),
                            }
                        }
                        if !is_editing {
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Expires" }
                                Select {
                                    class: Some("mt-1".into()),
                                    value: expires_in(),
                                    onchange: move |e: FormEvent| expires_in.set(e.value()),
                                    SelectOption { value: String::new(), "No expiry" }
                                    SelectOption { value: "604800".to_string(), "7 days" }
                                    SelectOption { value: "2592000".to_string(), "30 days" }
                                    SelectOption { value: "7776000".to_string(), "90 days" }
                                    SelectOption { value: "31536000".to_string(), "1 year" }
                                }
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()),
                                "Company (optional)"
                            }
                            Select {
                                value: company_id(),
                                onchange: move |e: FormEvent| company_id.set(e.value()),
                                SelectOption { value: String::new(), "No company" }
                                for c in companies() {
                                    SelectOption { value: c.id.clone(), "{c.label}" }
                                }
                            }
                        }
                        ApiAccessEditor {
                            draft: access,
                            projects: projects(),
                            compact: true,
                        }
                        div { class: "flex items-center gap-2",
                            Button {
                                disabled: creating(),
                                onclick: save,
                                if is_editing { "Save changes" } else { "Create" }
                            }
                            if is_editing {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| {
                                        editing_id.set(None);
                                        name.set(String::new());
                                        company_id.set(String::new());
                                        access.set(ApiAccessDraft::default());
                                    },
                                    "Cancel"
                                }
                            }
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
                                "No API keys yet. Create one so external apps can call your API projects."
                            }
                        }
                    } else {
                        div { class: "divide-y",
                            for row in keys() {
                                {
                                    let id = row.id.clone();
                                    let id_toggle = row.id.clone();
                                    let id_edit = row.id.clone();
                                    let name_edit = row.name.clone();
                                    let name_delete = row.name.clone();
                                    let enabled = row.enabled;
                                    let busy = busy_id().as_deref() == Some(row.id.as_str());
                                    let project_opts = projects();
                                    let scope_label = if row.project_ids.is_empty() {
                                        "All projects".to_string()
                                    } else {
                                        let labels: Vec<String> = row
                                            .project_ids
                                            .iter()
                                            .map(|pid| {
                                                project_opts
                                                    .iter()
                                                    .find(|p| &p.id == pid)
                                                    .map(|p| p.label.clone())
                                                    .unwrap_or_else(|| pid.clone())
                                            })
                                            .collect();
                                        labels.join(", ")
                                    };
                                    let call_bits = {
                                        let mut parts = Vec::new();
                                        if !row.endpoint_ids.is_empty() {
                                            parts.push(format!("{} endpoints", row.endpoint_ids.len()));
                                        }
                                        if !row.scopes_text.trim().is_empty() {
                                            parts.push("path scopes".into());
                                        }
                                        if parts.is_empty() {
                                            "all calls".into()
                                        } else {
                                            parts.join(" · ")
                                        }
                                    };
                                    let status_label = if enabled { "Enabled" } else { "Disabled" };
                                    let toggle_label = if enabled { "Disable" } else { "Enable" };
                                    let meta_for_edit = row.metadata.clone();
                                    let company_from_meta = meta_for_edit
                                        .get("companyId")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
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
                                                "{scope_label} · {call_bits} · {status_label}"
                                            }
                                            div { class: "text-xs text-muted-foreground",
                                                "Created {row.created} · Expires {row.expires}"
                                            }
                                            div { class: "ml-auto flex items-center gap-2",
                                                Button {
                                                    variant: ButtonVariant::Outline,
                                                    disabled: busy,
                                                    onclick: move |_| {
                                                        editing_id.set(Some(id_edit.clone()));
                                                        name.set(name_edit.clone());
                                                        company_id.set(company_from_meta.clone());
                                                        access.set(ApiAccessDraft::from_json(&meta_for_edit));
                                                    },
                                                    "Edit"
                                                }
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
                                                        confirm_delete.set(Some((id.clone(), name_delete.clone())));
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
