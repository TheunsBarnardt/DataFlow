use dioxus::prelude::*;

use crate::components::icons::{ChevronRight, Plus, Search};
use crate::components::panels::{json_bool, json_id, json_string};
use crate::components::ui::{PageLoader, Switch};

fn method_badge_class(method: &str) -> &'static str {
    match method.to_uppercase().as_str() {
        "GET" => "border-transparent bg-[#61affe] text-white",
        "POST" => "border-transparent bg-[#49cc90] text-white",
        "PUT" => "border-transparent bg-[#fca130] text-white",
        "PATCH" => "border-transparent bg-[#f39c12] text-white",
        "DELETE" => "border-transparent bg-[#f93e3e] text-white",
        "HEAD" => "border-transparent bg-[#c07a4a] text-white",
        "OPTIONS" => "border-transparent bg-[#0d5aa7] text-white",
        _ => "border-transparent bg-muted text-muted-foreground",
    }
}

fn auth_label(project: &serde_json::Value) -> String {
    let auth = project.get("auth");
    let scheme = auth
        .and_then(|a| a.get("schemeName"))
        .map(json_string)
        .unwrap_or_default();
    let mode = auth
        .and_then(|a| a.get("mode"))
        .map(json_string)
        .unwrap_or_default();
    if scheme == "ApiKey" {
        "API Key".into()
    } else if mode == "jwt" {
        "JWT".into()
    } else {
        "Open".into()
    }
}

fn format_updated(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    // Keep ISO readable; desktop has no locale formatter wired.
    raw.replace('T', " ").trim_end_matches('Z').to_string()
}

#[component]
pub fn ApiProjectsPanel(
    on_open_workflow: EventHandler<String>,
    #[props(default)] selected_project_id: Option<String>,
    on_selected_project_id_change: Option<EventHandler<Option<String>>>,
    #[props(default)] refresh_key: u32,
) -> Element {
    let mut projects = use_signal(Vec::<serde_json::Value>::new);
    let mut detail = use_signal(|| None::<serde_json::Value>);
    let mut err = use_signal(|| None::<String>);
    let mut list_loading = use_signal(|| true);
    let mut detail_loading = use_signal(|| false);
    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_slug = use_signal(String::new);
    let mut new_version = use_signal(|| "v1".to_string());
    let mut ep_path = use_signal(String::new);
    let mut ep_method = use_signal(|| "GET".to_string());
    let mut adding_endpoint = use_signal(|| false);
    let mut editing_version = use_signal(|| false);
    let mut version_draft = use_signal(String::new);
    let mut saving_version = use_signal(|| false);
    let mut endpoint_query = use_signal(String::new);

    let selected = selected_project_id.clone();

    // Props aren't reactive — mirror into signals so effects re-run on change.
    let mut selected_sig = use_signal(|| selected_project_id.clone());
    if *selected_sig.peek() != selected_project_id {
        selected_sig.set(selected_project_id.clone());
    }
    let mut refresh_sig = use_signal(|| refresh_key);
    if *refresh_sig.peek() != refresh_key {
        refresh_sig.set(refresh_key);
    }

    use_effect(move || {
        let _ = refresh_sig();
        list_loading.set(true);
        spawn(async move {
            match crate::api::get_json("/api/api-projects").await {
                Ok(v) => {
                    projects.set(
                        v.get("projects")
                            .and_then(|p| p.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            list_loading.set(false);
        });
    });

    use_effect(move || {
        let selected_for_load = selected_sig();
        editing_version.set(false);
        version_draft.set(String::new());
        endpoint_query.set(String::new());
        if let Some(id) = selected_for_load {
            detail.set(None);
            detail_loading.set(true);
            spawn(async move {
                match crate::api::get_json(&format!("/api/api-projects/{id}")).await {
                    Ok(v) => {
                        detail.set(Some(v));
                        err.set(None);
                    }
                    Err(e) => {
                        detail.set(None);
                        err.set(Some(e));
                    }
                }
                detail_loading.set(false);
            });
        } else {
            detail.set(None);
            detail_loading.set(false);
        }
    });

    let set_selected = move |id: Option<String>| {
        if let Some(cb) = on_selected_project_id_change.as_ref() {
            cb.call(id);
        }
    };

    if selected.is_some() && detail_loading() {
        return rsx! {
            PageLoader { label: "Loading API project…" }
        };
    }

    if let (Some(sel), Some(d)) = (selected.clone(), detail()) {
        let project = d
            .get("project")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let endpoints = d
            .get("endpoints")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();
        let name = project.get("name").map(json_string).unwrap_or_default();
        let slug = project.get("slug").map(json_string).unwrap_or_default();
        let version = project
            .get("version")
            .map(json_string)
            .unwrap_or_else(|| "v1".into());
        let auth = auth_label(&project);
        let base = crate::api::base_url();
        let openapi_url = format!("{base}/apis/{slug}/openapi.json");
        let swagger_url = format!("{base}/apis/{slug}/swagger");
        let q = endpoint_query().trim().to_lowercase();
        let filtered: Vec<serde_json::Value> = endpoints
            .iter()
            .cloned()
            .filter(|ep| {
                if q.is_empty() {
                    return true;
                }
                let hay = format!(
                    "{} {} {} {} {} {}",
                    ep.get("method").map(json_string).unwrap_or_default(),
                    ep.get("path").map(json_string).unwrap_or_default(),
                    ep.get("summary").map(json_string).unwrap_or_default(),
                    ep.get("tag").map(json_string).unwrap_or_default(),
                    ep.get("auth").map(json_string).unwrap_or_default(),
                    version,
                );
                hay.to_lowercase().contains(&q)
            })
            .collect();
        let count_label = if endpoint_query().trim().is_empty() {
            let n = endpoints.len();
            format!("{n} endpoint{}", if n == 1 { "" } else { "s" })
        } else {
            format!("{} of {}", filtered.len(), endpoints.len())
        };
        let sel_for_version = sel.clone();
        let sel_for_add = sel.clone();
        let version_for_edit = version.clone();

        return rsx! {
            div { class: "flex h-full flex-col",
                div { class: "flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5",
                    div {
                        button {
                            r#type: "button",
                            class: "mb-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground",
                            onclick: move |_| set_selected(None),
                            "← All API projects"
                        }
                        h1 { class: "text-2xl font-semibold tracking-tight", "{name}" }
                        div { class: "mt-1 flex flex-wrap items-center gap-2 text-sm text-muted-foreground",
                            span { "/apis/{slug}" }
                            span { "·" }
                            if editing_version() {
                                span { class: "inline-flex items-center gap-1.5",
                                    input {
                                        class: "flex h-7 w-20 rounded-md border border-input bg-transparent px-2 text-sm",
                                        value: "{version_draft()}",
                                        oninput: move |e| version_draft.set(e.value()),
                                    }
                                    button {
                                        r#type: "button",
                                        class: "inline-flex h-7 items-center rounded-md bg-primary px-2 text-xs text-primary-foreground disabled:opacity-50",
                                        disabled: saving_version(),
                                        onclick: move |_| {
                                            let id = sel_for_version.clone();
                                            let next = {
                                                let t = version_draft().trim().to_string();
                                                if t.is_empty() { "v1".into() } else { t }
                                            };
                                            saving_version.set(true);
                                            spawn(async move {
                                                let body = serde_json::json!({ "version": next });
                                                match crate::api::put_json(
                                                    &format!("/api/api-projects/{id}"),
                                                    body,
                                                )
                                                .await
                                                {
                                                    Ok(_) => {
                                                        editing_version.set(false);
                                                        if let Ok(v) = crate::api::get_json(
                                                            &format!("/api/api-projects/{id}"),
                                                        )
                                                        .await
                                                        {
                                                            detail.set(Some(v));
                                                        }
                                                        if let Ok(reload) =
                                                            crate::api::get_json("/api/api-projects")
                                                                .await
                                                        {
                                                            projects.set(
                                                                reload
                                                                    .get("projects")
                                                                    .and_then(|p| p.as_array())
                                                                    .cloned()
                                                                    .unwrap_or_default(),
                                                            );
                                                        }
                                                    }
                                                    Err(e) => err.set(Some(e)),
                                                }
                                                saving_version.set(false);
                                            });
                                        },
                                        "Save"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "inline-flex h-7 items-center rounded-md px-2 text-xs text-muted-foreground hover:bg-muted",
                                        disabled: saving_version(),
                                        onclick: move |_| editing_version.set(false),
                                        "Cancel"
                                    }
                                }
                            } else {
                                button {
                                    r#type: "button",
                                    class: "rounded px-1 font-medium text-foreground hover:bg-muted",
                                    title: "Edit version",
                                    onclick: move |_| {
                                        version_draft.set(version_for_edit.clone());
                                        editing_version.set(true);
                                    },
                                    "{version}"
                                }
                            }
                            span { "·" }
                            span { "{auth} auth" }
                        }
                    }
                    div { class: "flex items-center gap-2",
                        a {
                            href: "{openapi_url}",
                            target: "_blank",
                            class: "inline-flex h-9 items-center gap-2 rounded-md border bg-background px-4 text-sm hover:bg-accent",
                            "OpenAPI"
                        }
                        a {
                            href: "{swagger_url}",
                            target: "_blank",
                            class: "inline-flex h-9 items-center gap-2 rounded-md bg-primary px-4 text-sm text-primary-foreground hover:bg-primary/90",
                            "Swagger UI"
                        }
                    }
                }

                div { class: "flex flex-wrap items-end gap-2 border-b px-6 py-3",
                    div { class: "min-w-[200px] flex-1",
                        label { class: "text-xs text-muted-foreground", "New path" }
                        input {
                            class: "mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                            placeholder: "Controller/Action",
                            value: "{ep_path()}",
                            oninput: move |e| ep_path.set(e.value()),
                        }
                    }
                    div {
                        label { class: "text-xs text-muted-foreground", "Method" }
                        select {
                            class: "mt-1 flex h-9 w-[7.5rem] rounded-md border border-input bg-transparent px-2 text-sm",
                            value: "{ep_method()}",
                            onchange: move |e| ep_method.set(e.value()),
                            option { value: "GET", "GET" }
                            option { value: "POST", "POST" }
                            option { value: "PUT", "PUT" }
                            option { value: "PATCH", "PATCH" }
                            option { value: "DELETE", "DELETE" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-9 items-center gap-2 rounded-md bg-primary px-4 text-sm text-primary-foreground disabled:opacity-50",
                        disabled: adding_endpoint(),
                        onclick: move |_| {
                            let id = sel_for_add.clone();
                            let path = ep_path().trim().to_string();
                            let method = ep_method();
                            adding_endpoint.set(true);
                            spawn(async move {
                                let body = serde_json::json!({
                                    "path": if path.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(path) },
                                    "method": method,
                                });
                                match crate::api::post_json(
                                    &format!("/api/api-projects/{id}/endpoints"),
                                    body,
                                )
                                .await
                                {
                                    Ok(v) => {
                                        ep_path.set(String::new());
                                        if let Ok(d) = crate::api::get_json(
                                            &format!("/api/api-projects/{id}"),
                                        )
                                        .await
                                        {
                                            detail.set(Some(d));
                                        }
                                        if let Some(wid) = v
                                            .get("workflow")
                                            .map(json_id)
                                            .filter(|s| !s.is_empty())
                                        {
                                            on_open_workflow.call(wid);
                                        }
                                    }
                                    Err(e) => err.set(Some(e)),
                                }
                                adding_endpoint.set(false);
                            });
                        },
                        Plus { class: Some("h-4 w-4".into()) }
                        "Add endpoint"
                    }
                }

                if let Some(message) = err() {
                    div { class: "border-b bg-destructive/10 px-6 py-2 text-sm text-destructive", "{message}" }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto",
                    div { class: "p-6",
                        div { class: "mb-3 flex flex-wrap items-center justify-between gap-2",
                            div { class: "relative min-w-[200px] max-w-md flex-1",
                                Search { class: Some("pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground".into()) }
                                input {
                                    class: "flex h-9 w-full rounded-md border border-input bg-transparent pl-8 pr-3 text-sm",
                                    placeholder: "Search endpoints...",
                                    value: "{endpoint_query()}",
                                    oninput: move |e| endpoint_query.set(e.value()),
                                }
                            }
                            div { class: "text-xs text-muted-foreground", "{count_label}" }
                        }
                        div { class: "overflow-hidden rounded-lg border bg-card",
                            div { class: "grid grid-cols-[90px_1.4fr_1fr_80px_100px_80px] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                                div { "Method" }
                                div { "Path" }
                                div { "Summary" }
                                div { "Version" }
                                div { "Auth" }
                                div {}
                            }
                            for ep in filtered.iter().cloned() {
                                {
                                    let eid = json_id(&ep);
                                    let method = ep.get("method").map(json_string).unwrap_or_else(|| "GET".into());
                                    let path = ep.get("path").map(json_string).unwrap_or_default();
                                    let summary = ep
                                        .get("summary")
                                        .or_else(|| ep.get("tag"))
                                        .map(json_string)
                                        .unwrap_or_default();
                                    let ep_auth = ep.get("auth").map(json_string).unwrap_or_default();
                                    let wid = ep
                                        .get("workflowId")
                                        .map(json_string)
                                        .unwrap_or_default();
                                    let badge = method_badge_class(&method);
                                    let ver = version.clone();
                                    rsx! {
                                        button {
                                            key: "{eid}",
                                            r#type: "button",
                                            class: "grid w-full grid-cols-[90px_1.4fr_1fr_80px_100px_80px] gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50",
                                            onclick: move |_| {
                                                if !wid.is_empty() {
                                                    on_open_workflow.call(wid.clone());
                                                }
                                            },
                                            div {
                                                span { class: "inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-medium {badge}",
                                                    "{method}"
                                                }
                                            }
                                            div { class: "truncate font-medium", "{path}" }
                                            div { class: "truncate text-muted-foreground", "{summary}" }
                                            div { class: "text-xs text-muted-foreground", "{ver}" }
                                            div { class: "text-xs text-muted-foreground", "{ep_auth}" }
                                            div { class: "flex justify-end text-muted-foreground",
                                                ChevronRight { class: Some("h-4 w-4".into()) }
                                            }
                                        }
                                    }
                                }
                            }
                            if endpoints.is_empty() {
                                div { class: "px-4 py-8 text-center text-sm text-muted-foreground",
                                    "No endpoints yet — add one above."
                                }
                            } else if filtered.is_empty() {
                                div { class: "px-4 py-8 text-center text-sm text-muted-foreground",
                                    "No endpoints match \"{endpoint_query().trim()}\"."
                                }
                            }
                        }
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "flex h-full flex-col",
            div { class: "flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5",
                div {
                    h1 { class: "text-2xl font-semibold tracking-tight", "API Projects" }
                    p { class: "mt-1 text-sm text-muted-foreground",
                        "Multi-endpoint APIs with Swagger — each route is a flow"
                    }
                }
            }

            div { class: "flex flex-wrap items-end gap-2 border-b px-6 py-3",
                div { class: "min-w-[180px]",
                    label { class: "text-xs text-muted-foreground", "Name" }
                    input {
                        class: "mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                        placeholder: "My API",
                        value: "{new_name()}",
                        oninput: move |e| new_name.set(e.value()),
                    }
                }
                div { class: "min-w-[140px]",
                    label { class: "text-xs text-muted-foreground", "Slug" }
                    input {
                        class: "mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                        placeholder: "my-api",
                        value: "{new_slug()}",
                        oninput: move |e| new_slug.set(e.value()),
                    }
                }
                div { class: "w-[100px]",
                    label { class: "text-xs text-muted-foreground", "Version" }
                    input {
                        class: "mt-1 flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                        placeholder: "v1",
                        value: "{new_version()}",
                        oninput: move |e| new_version.set(e.value()),
                    }
                }
                button {
                    r#type: "button",
                    class: "inline-flex h-9 items-center gap-2 rounded-md bg-primary px-4 text-sm text-primary-foreground disabled:opacity-50",
                    disabled: creating(),
                    onclick: move |_| {
                        creating.set(true);
                        let name = new_name().trim().to_string();
                        let slug = new_slug().trim().to_string();
                        let version = {
                            let t = new_version().trim().to_string();
                            if t.is_empty() { "v1".into() } else { t }
                        };
                        spawn(async move {
                            let body = serde_json::json!({
                                "name": if name.is_empty() { "New API" } else { name.as_str() },
                                "slug": if slug.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(slug) },
                                "version": version,
                                "auth": { "mode": "jwt" }
                            });
                            match crate::api::post_json("/api/api-projects", body).await {
                                Ok(v) => {
                                    new_name.set(String::new());
                                    new_slug.set(String::new());
                                    new_version.set("v1".into());
                                    if let Some(pid) =
                                        v.get("project").map(json_id).filter(|s| !s.is_empty())
                                    {
                                        set_selected(Some(pid));
                                    }
                                    if let Ok(reload) =
                                        crate::api::get_json("/api/api-projects").await
                                    {
                                        projects.set(
                                            reload
                                                .get("projects")
                                                .and_then(|p| p.as_array())
                                                .cloned()
                                                .unwrap_or_default(),
                                        );
                                    }
                                }
                                Err(e) => err.set(Some(e)),
                            }
                            creating.set(false);
                        });
                    },
                    Plus { class: Some("h-4 w-4".into()) }
                    "Create API"
                }
            }

            if let Some(message) = err() {
                div { class: "border-b bg-destructive/10 px-6 py-2 text-sm text-destructive", "{message}" }
            }

            if list_loading() {
                PageLoader { label: "Loading API projects…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    div { class: "overflow-hidden rounded-lg border bg-card",
                        div { class: "grid grid-cols-[minmax(0,1.5fr)_120px_80px_100px_140px_auto] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                            div { "Name" }
                            div { "Slug" }
                            div { "Version" }
                            div { "Status" }
                            div { "Updated" }
                            div { class: "text-right", "Swagger" }
                        }
                        for p in projects().into_iter() {
                            {
                                let id = json_id(&p);
                                let name = p.get("name").map(json_string).unwrap_or_default();
                                let slug = p.get("slug").map(json_string).unwrap_or_default();
                                let version = p
                                    .get("version")
                                    .map(json_string)
                                    .unwrap_or_else(|| "v1".into());
                                let enabled = json_bool(&p, "enabled", true);
                                let updated = format_updated(
                                    &p.get("updatedAt").map(json_string).unwrap_or_default(),
                                );
                                let id_sel = id.clone();
                                let id_toggle = id.clone();
                                let swagger = format!(
                                    "{}/apis/{}/swagger",
                                    crate::api::base_url(),
                                    slug
                                );
                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "grid w-full cursor-pointer grid-cols-[minmax(0,1.5fr)_120px_80px_100px_140px_auto] items-center gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50",
                                        onclick: move |_| set_selected(Some(id_sel.clone())),
                                        div { class: "truncate font-medium", "{name}" }
                                        div { class: "truncate text-muted-foreground", "{slug}" }
                                        div { class: "truncate text-muted-foreground", "{version}" }
                                        div {
                                            onclick: move |e| e.stop_propagation(),
                                            Switch {
                                                checked: enabled,
                                                size: Some("sm".into()),
                                                on_checked_change: move |checked: bool| {
                                                    let id = id_toggle.clone();
                                                    spawn(async move {
                                                        let _ = crate::api::put_json(
                                                            &format!("/api/api-projects/{id}"),
                                                            serde_json::json!({ "enabled": checked }),
                                                        )
                                                        .await;
                                                        if let Ok(reload) =
                                                            crate::api::get_json("/api/api-projects")
                                                                .await
                                                        {
                                                            projects.set(
                                                                reload
                                                                    .get("projects")
                                                                    .and_then(|p| p.as_array())
                                                                    .cloned()
                                                                    .unwrap_or_default(),
                                                            );
                                                        }
                                                    });
                                                },
                                            }
                                        }
                                        div { class: "truncate text-xs text-muted-foreground", "{updated}" }
                                        a {
                                            href: "{swagger}",
                                            target: "_blank",
                                            class: "inline-flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-foreground",
                                            onclick: move |e| e.stop_propagation(),
                                            "Open Swagger"
                                        }
                                    }
                                }
                            }
                        }
                        if projects().is_empty() {
                            div { class: "px-4 py-8 text-center text-sm text-muted-foreground",
                                "No API projects yet. Create one, or start the server to seed Wealth."
                            }
                        }
                    }
                }
            }
            }
        }
    }
}
