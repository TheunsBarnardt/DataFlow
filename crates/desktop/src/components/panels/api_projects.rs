use dioxus::prelude::*;

use crate::components::icons::{ChevronRight, MoreHorizontal, Pencil, Plus, Search, Trash2};
use crate::components::panels::{json_bool, json_id, json_string};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonSize, ButtonVariant, Card,
    ConfirmDeleteDialog, Empty, EmptyDescription, EmptyHeader, Input, Label, PageLoader, Switch,
};

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
    crate::components::panels::format_local_datetime(raw)
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
    let mut menu_for = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);
    let mut editing_project = use_signal(|| None::<(String, String, String)>);
    let mut busy = use_signal(|| false);

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
            let clear_selected = on_selected_project_id_change;
            spawn(async move {
                match crate::api::get_json(&format!("/api/api-projects/{id}")).await {
                    Ok(v) => {
                        detail.set(Some(v));
                        err.set(None);
                    }
                    Err(e) => {
                        detail.set(None);
                        // Stale selection after reimport (old project id) — drop it
                        // instead of leaving the list page stuck on "Not found".
                        if e == "Not found" {
                            err.set(None);
                            if let Some(cb) = clear_selected {
                                cb.call(None);
                            }
                        } else {
                            err.set(Some(e));
                        }
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
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: Some("mb-2 h-auto gap-1 px-0 text-xs text-muted-foreground".into()),
                            onclick: move |_| set_selected(None),
                            "← All API projects"
                        }
                        h1 { class: "text-2xl font-semibold tracking-tight", "{name}" }
                        div { class: "mt-1 flex flex-wrap items-center gap-2 text-sm text-muted-foreground",
                            span { "/apis/{slug}" }
                            span { "·" }
                            if editing_version() {
                                span { class: "inline-flex items-center gap-1.5",
                                    Input {
                                        class: Some("h-7 w-20 px-2".into()),
                                        value: version_draft(),
oninput: move |e: FormEvent| version_draft.set(e.value()),
                                    }
                                    Button {
                                        size: ButtonSize::Sm,
                                        class: Some("h-7 px-2 text-xs".into()),
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
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        size: ButtonSize::Sm,
                                        class: Some("h-7 px-2 text-xs".into()),
                                        disabled: saving_version(),
                                        onclick: move |_| editing_version.set(false),
                                        "Cancel"
                                    }
                                }
                            } else {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: Some("h-auto rounded px-1 font-medium".into()),
                                    title: Some("Edit version".into()),
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
                    div { class: "min-w-[200px] flex-1 flex flex-col gap-1",
                        Label { class: Some("text-xs text-muted-foreground".into()), "New path" }
                        Input {
                            placeholder: Some("Controller/Action".into()),
                            value: ep_path(),
                            oninput: move |e: FormEvent| ep_path.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        Label { class: Some("text-xs text-muted-foreground".into()), "Method" }
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
                    Button {
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
                    Alert {
                        variant: AlertVariant::Destructive,
                        class: Some("rounded-none border-x-0 border-t-0 bg-destructive/10".into()),
                        AlertDescription {
                            class: Some("text-destructive".into()),
                            "{message}"
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto",
                    div { class: "p-6",
                        div { class: "mb-3 flex flex-wrap items-center justify-between gap-2",
                            div { class: "relative min-w-[200px] max-w-md flex-1",
                                Search { class: Some("pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground".into()) }
                                Input {
                                    class: Some("pl-8".into()),
                                    placeholder: Some("Search endpoints...".into()),
                                    value: endpoint_query(),
                                    oninput: move |e: FormEvent| endpoint_query.set(e.value()),
                                }
                            }
                            div { class: "text-xs text-muted-foreground", "{count_label}" }
                        }
                        Card {
                            class: Some("overflow-hidden p-0 gap-0 shadow-sm".into()),
                            div {
                                class: "flex w-full items-center gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                                div { class: "shrink-0", style: "width:90px", "Method" }
                                div { class: "min-w-0 flex-1", "Path" }
                                div { class: "min-w-0 flex-1", "Summary" }
                                div { class: "shrink-0", style: "width:80px", "Version" }
                                div { class: "shrink-0", style: "width:100px", "Auth" }
                                div { class: "w-10 shrink-0" }
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
                                            class: "flex w-full items-center gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50",
                                            onclick: move |_| {
                                                if !wid.is_empty() {
                                                    on_open_workflow.call(wid.clone());
                                                }
                                            },
                                            div { class: "shrink-0", style: "width:90px",
                                                span { class: "inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-medium {badge}",
                                                    "{method}"
                                                }
                                            }
                                            div { class: "min-w-0 flex-1 truncate font-medium", "{path}" }
                                            div { class: "min-w-0 flex-1 truncate text-muted-foreground", "{summary}" }
                                            div { class: "shrink-0 text-xs text-muted-foreground", style: "width:80px", "{ver}" }
                                            div { class: "shrink-0 text-xs text-muted-foreground", style: "width:100px", "{ep_auth}" }
                                            div { class: "flex w-10 shrink-0 justify-end text-muted-foreground",
                                                ChevronRight { class: Some("h-4 w-4".into()) }
                                            }
                                        }
                                    }
                                }
                            }
                            if endpoints.is_empty() {
                                Empty {
                                    class: Some("border-0 px-4 py-8".into()),
                                    EmptyHeader {
                                        EmptyDescription { "No endpoints yet — add one above." }
                                    }
                                }
                            } else if filtered.is_empty() {
                                Empty {
                                    class: Some("border-0 px-4 py-8".into()),
                                    EmptyHeader {
                                        EmptyDescription {
                                            "No endpoints match \"{endpoint_query().trim()}\"."
                                        }
                                    }
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
                div { class: "min-w-[180px] flex flex-col gap-1",
                    Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                    Input {
                        placeholder: Some("My API".into()),
                        value: new_name(),
                        oninput: move |e: FormEvent| new_name.set(e.value()),
                    }
                }
                div { class: "min-w-[140px] flex flex-col gap-1",
                    Label { class: Some("text-xs text-muted-foreground".into()), "Slug" }
                    Input {
                        placeholder: Some("my-api".into()),
                        value: new_slug(),
                        oninput: move |e: FormEvent| new_slug.set(e.value()),
                    }
                }
                div { class: "w-[100px] flex flex-col gap-1",
                    Label { class: Some("text-xs text-muted-foreground".into()), "Version" }
                    Input {
                        placeholder: Some("v1".into()),
                        value: new_version(),
                        oninput: move |e: FormEvent| new_version.set(e.value()),
                    }
                }
                Button {
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
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("rounded-none border-x-0 border-t-0 bg-destructive/10".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            if list_loading() {
                PageLoader { label: "Loading API projects…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    Card {
                        class: Some(if menu_for().is_some() {
                            "relative z-50 overflow-visible p-0 gap-0 shadow-sm".into()
                        } else {
                            "overflow-hidden p-0 gap-0 shadow-sm".into()
                        }),
                        div {
                            class: "flex w-full items-center gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                            div { class: "min-w-0 flex-1", "Name" }
                            div { class: "shrink-0", style: "width:120px", "Slug" }
                            div { class: "shrink-0", style: "width:80px", "Version" }
                            div { class: "shrink-0", style: "width:100px", "Status" }
                            div { class: "shrink-0", style: "width:140px", "Updated" }
                            div { class: "shrink-0 text-right", style: "width:110px", "Swagger" }
                            div { class: "w-10 shrink-0" }
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
                                let id_menu = id.clone();
                                let id_ctx = id.clone();
                                let menu_open = menu_for().as_deref() == Some(id.as_str());
                                let row_class = if menu_open {
                                    "relative z-10 flex w-full cursor-pointer items-center gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50"
                                } else {
                                    "relative flex w-full cursor-pointer items-center gap-3 border-b px-4 py-3 text-left text-sm last:border-b-0 hover:bg-muted/50"
                                };
                                let edit_id = id.clone();
                                let edit_name = name.clone();
                                let edit_slug = slug.clone();
                                let del_id = id.clone();
                                let del_name = name.clone();
                                let swagger = format!(
                                    "{}/apis/{}/swagger",
                                    crate::api::base_url(),
                                    slug
                                );
                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "{row_class}",
                                        onclick: move |_| set_selected(Some(id_sel.clone())),
                                        oncontextmenu: move |e| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            menu_for.set(Some(id_ctx.clone()));
                                        },
                                        div { class: "min-w-0 flex-1 truncate font-medium", "{name}" }
                                        div { class: "shrink-0 truncate text-muted-foreground", style: "width:120px", "{slug}" }
                                        div { class: "shrink-0 truncate text-muted-foreground", style: "width:80px", "{version}" }
                                        div {
                                            class: "shrink-0",
                                            style: "width:100px",
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
                                        div { class: "shrink-0 truncate text-xs text-muted-foreground", style: "width:140px", "{updated}" }
                                        a {
                                            href: "{swagger}",
                                            target: "_blank",
                                            class: "inline-flex shrink-0 items-center justify-end gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-foreground",
                                            style: "width:110px",
                                            onclick: move |e| e.stop_propagation(),
                                            "Open Swagger"
                                        }
                                        div {
                                            class: "relative flex w-10 shrink-0 justify-end",
                                            onclick: move |e| e.stop_propagation(),
                                            button {
                                                r#type: "button",
                                                class: "rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground",
                                                title: "More",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    if menu_for().as_deref() == Some(id_menu.as_str()) {
                                                        menu_for.set(None);
                                                    } else {
                                                        menu_for.set(Some(id_menu.clone()));
                                                    }
                                                },
                                                MoreHorizontal { class: Some("h-4 w-4".into()) }
                                            }
                                            if menu_open {
                                                div {
                                                    class: "absolute right-0 top-full z-50 mt-1 min-w-[10rem] rounded-md border bg-popover p-1 text-popover-foreground text-sm shadow-md",
                                                    onclick: move |e| e.stop_propagation(),
                                                    button {
                                                        r#type: "button",
                                                        class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-accent",
                                                        onclick: move |_| {
                                                            menu_for.set(None);
                                                            editing_project.set(Some((
                                                                edit_id.clone(),
                                                                edit_name.clone(),
                                                                edit_slug.clone(),
                                                            )));
                                                        },
                                                        Pencil { class: Some("h-4 w-4".into()) }
                                                        "Edit"
                                                    }
                                                    button {
                                                        r#type: "button",
                                                        class: "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-destructive hover:bg-destructive/10",
                                                        onclick: move |_| {
                                                            menu_for.set(None);
                                                            confirm_delete.set(Some((
                                                                del_id.clone(),
                                                                del_name.clone(),
                                                            )));
                                                        },
                                                        Trash2 { class: Some("h-4 w-4".into()) }
                                                        "Delete"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if projects().is_empty() {
                            Empty {
                                class: Some("border-0 px-4 py-8".into()),
                                EmptyHeader {
                                    EmptyDescription {
                                        "No API projects yet. Create one, or start the server to seed Wealth."
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }

            if menu_for().is_some() {
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| menu_for.set(None),
                }
            }

            if let Some((edit_id, edit_name, edit_slug)) = editing_project() {
                {
                    let edit_id_save = edit_id.clone();
                    let edit_id_name = edit_id.clone();
                    let edit_id_slug = edit_id.clone();
                    rsx! {
                        div {
                            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4 backdrop-blur-[1px]",
                            onclick: move |_| editing_project.set(None),
                            div {
                                class: "w-full max-w-sm rounded-xl border bg-card p-5 shadow-2xl",
                                onclick: move |e| e.stop_propagation(),
                                role: "dialog",
                                "aria-modal": "true",
                                h2 { class: "text-lg font-semibold", "Edit API project" }
                                p { class: "mt-2 text-sm text-muted-foreground",
                                    "Update the display name and slug."
                                }
                                div { class: "mt-4 flex flex-col gap-3",
                                    div { class: "flex flex-col gap-1",
                                        Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                                        Input {
                                            value: edit_name.clone(),
                                            oninput: move |e: FormEvent| {
                                                let slug = editing_project()
                                                    .map(|(_, _, s)| s)
                                                    .unwrap_or_default();
                                                editing_project.set(Some((
                                                    edit_id_name.clone(),
                                                    e.value(),
                                                    slug,
                                                )));
                                            },
                                        }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        Label { class: Some("text-xs text-muted-foreground".into()), "Slug" }
                                        Input {
                                            value: edit_slug.clone(),
                                            oninput: move |e: FormEvent| {
                                                let name = editing_project()
                                                    .map(|(_, n, _)| n)
                                                    .unwrap_or_default();
                                                editing_project.set(Some((
                                                    edit_id_slug.clone(),
                                                    name,
                                                    e.value(),
                                                )));
                                            },
                                        }
                                    }
                                }
                                div { class: "mt-5 flex justify-end gap-2",
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        disabled: busy(),
                                        onclick: move |_| editing_project.set(None),
                                        "Cancel"
                                    }
                                    Button {
                                        disabled: busy(),
                                        onclick: move |_| {
                                            let id = edit_id_save.clone();
                                            let (name, slug) = editing_project()
                                                .map(|(_, n, s)| {
                                                    (n.trim().to_string(), s.trim().to_string())
                                                })
                                                .unwrap_or_default();
                                            if name.is_empty() {
                                                return;
                                            }
                                            busy.set(true);
                                            spawn(async move {
                                                let mut body = serde_json::json!({ "name": name });
                                                if !slug.is_empty() {
                                                    body["slug"] = serde_json::Value::String(slug);
                                                }
                                                match crate::api::put_json(
                                                    &format!("/api/api-projects/{id}"),
                                                    body,
                                                )
                                                .await
                                                {
                                                    Ok(_) => {
                                                        editing_project.set(None);
                                                        err.set(None);
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
                                                busy.set(false);
                                            });
                                        },
                                        "Save"
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
                        "This will permanently delete \"{del_name}\" and all of its endpoints. This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete API project?".to_string(),
                            description,
                            busy: busy(),
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                let id = del_id_confirm.clone();
                                busy.set(true);
                                spawn(async move {
                                    match crate::api::delete_json(&format!("/api/api-projects/{id}"))
                                        .await
                                    {
                                        Ok(_) => {
                                            confirm_delete.set(None);
                                            err.set(None);
                                            if selected_sig().as_deref() == Some(id.as_str()) {
                                                set_selected(None);
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
                                    busy.set(false);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}
