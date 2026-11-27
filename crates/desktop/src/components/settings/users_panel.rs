use dioxus::prelude::*;

use super::api_access_editor::{
    parse_projects, ApiAccessDraft, ApiAccessEditor, ProjectOpt,
};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonVariant, Card, CardContent, CardHeader,
    CardTitle, DataTable, DataTableBody, DataTableCell, DataTableHead, DataTableHeader, DataTableRow,
    Empty, EmptyDescription, Input, Label, Select, SelectOption, Switch,
};
use crate::roles::DATAFLOW_ROLES;

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .or_else(|| v.get(key).map(|x| x.to_string()))
        .unwrap_or_default()
}

fn user_id(v: &serde_json::Value) -> String {
    json_str(v, "id")
}

fn user_active(v: &serde_json::Value) -> bool {
    let status = json_str(v, "status");
    if status.is_empty() {
        !v.get("banned").and_then(|x| x.as_bool()).unwrap_or(false)
    } else {
        status == "active"
    }
}

#[component]
fn RoleSelect(
    value: String,
    class: Option<String>,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        Select {
            class: class,
            value: value.clone(),
            onchange: move |e: FormEvent| on_change.call(e.value()),
            for role in DATAFLOW_ROLES.iter() {
                SelectOption {
                    value: (*role).to_string(),
                    selected: value == *role,
                    "{role}"
                }
            }
        }
    }
}

#[component]
pub fn UsersPanel() -> Element {
    let mut users = use_signal(Vec::<serde_json::Value>::new);
    let mut projects = use_signal(Vec::<ProjectOpt>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let mut email = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut role = use_signal(|| "editor".to_string());
    let mut company_name = use_signal(String::new);
    let mut override_access = use_signal(|| false);
    let mut access = use_signal(ApiAccessDraft::default);
    let mut editing_id = use_signal(|| None::<String>);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            let users_res = crate::api::get_json("/api/users").await;
            let projects_res = crate::api::get_json("/api/api-projects").await;
            match users_res {
                Ok(v) => {
                    users.set(
                        v.get("users")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
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

    let create_user = move |_| {
        let body = {
            let mut body = serde_json::json!({
                "email": email(),
                "username": username(),
                "name": name(),
                "password": password(),
                "role": role(),
                "companyName": company_name(),
            });
            if override_access() {
                body["apiAccess"] = access().to_json();
            }
            body
        };
        busy.set(true);
        spawn(async move {
            match crate::api::post_json("/api/users", body).await {
                Ok(_) => {
                    email.set(String::new());
                    username.set(String::new());
                    name.set(String::new());
                    password.set(String::new());
                    company_name.set(String::new());
                    override_access.set(false);
                    access.set(ApiAccessDraft::default());
                    err.set(None);
                    if let Ok(v) = crate::api::get_json("/api/users").await {
                        users.set(
                            v.get("users")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                }
                Err(e) => err.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let save_override = move |_| {
        let Some(uid) = editing_id() else {
            return;
        };
        busy.set(true);
        let use_override = override_access();
        let draft = access();
        spawn(async move {
            let body = if use_override {
                serde_json::json!({ "apiAccess": draft.to_json() })
            } else {
                serde_json::json!({ "clearApiAccess": true })
            };
            match crate::api::patch_json(&format!("/api/users/{uid}"), body).await {
                Ok(_) => {
                    editing_id.set(None);
                    override_access.set(false);
                    access.set(ApiAccessDraft::default());
                    err.set(None);
                    if let Ok(v) = crate::api::get_json("/api/users").await {
                        users.set(
                            v.get("users")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                }
                Err(e) => err.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let is_editing = editing_id().is_some();

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Users" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Create accounts, assign roles, and optionally override API call access per user."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            div { class: "grid min-h-0 flex-1 gap-6 p-6 lg:grid-cols-[minmax(320px,420px)_1fr]",
                Card {
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()),
                            if is_editing { "Edit API access override" } else { "Create user" }
                        }
                    }
                    CardContent { class: Some("space-y-3 px-4 pb-4".into()),
                        if !is_editing {
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Display name" }
                                Input {
                                    placeholder: Some("Display name".into()),
                                    value: name(),
                                    oninput: move |e: FormEvent| name.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Username" }
                                Input {
                                    placeholder: Some("Username".into()),
                                    value: username(),
                                    oninput: move |e: FormEvent| username.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Email" }
                                Input {
                                    placeholder: Some("Email".into()),
                                    r#type: Some("email".into()),
                                    value: email(),
                                    oninput: move |e: FormEvent| email.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Password" }
                                Input {
                                    placeholder: Some("Password".into()),
                                    r#type: Some("password".into()),
                                    value: password(),
                                    oninput: move |e: FormEvent| password.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Company" }
                                Input {
                                    placeholder: Some("Company (optional)".into()),
                                    value: company_name(),
                                    oninput: move |e: FormEvent| company_name.set(e.value()),
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                Label { class: Some("text-xs text-muted-foreground".into()), "Role" }
                                RoleSelect {
                                    value: role(),
                                    class: None,
                                    on_change: move |r| role.set(r),
                                }
                            }
                        }
                        div { class: "flex items-center gap-2 rounded-md border p-2",
                            Switch {
                                checked: override_access(),
                                on_checked_change: move |v| override_access.set(v),
                            }
                            Label { class: Some("text-sm".into()),
                                "Custom API call access (override role)"
                            }
                        }
                        if override_access() {
                            ApiAccessEditor {
                                draft: access,
                                projects: projects(),
                                compact: true,
                            }
                        }
                        if is_editing {
                            div { class: "flex items-center gap-2",
                                Button {
                                    disabled: busy(),
                                    onclick: save_override,
                                    "Save override"
                                }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| {
                                        editing_id.set(None);
                                        override_access.set(false);
                                        access.set(ApiAccessDraft::default());
                                    },
                                    "Cancel"
                                }
                            }
                        } else {
                            Button {
                                disabled: busy(),
                                onclick: create_user,
                                "Create"
                            }
                        }
                    }
                }

                Card { class: Some("min-w-0 overflow-auto p-0 gap-0".into()),
                    if loading() {
                        CardContent { class: Some("p-4 text-sm text-muted-foreground".into()), "Loading…" }
                    } else if users().is_empty() {
                        Empty { class: Some("border-0 py-10".into()),
                            EmptyDescription { "No users" }
                        }
                    } else {
                        DataTable { class: Some("max-w-none".into()),
                            DataTableHeader { class: Some("bg-muted/40 text-xs".into()),
                                DataTableRow {
                                    DataTableHead { class: Some("px-3 py-2".into()), "User" }
                                    DataTableHead { class: Some("px-3 py-2".into()), "Role" }
                                    DataTableHead { class: Some("px-3 py-2".into()), "API access" }
                                    DataTableHead { class: Some("px-3 py-2".into()), "Active" }
                                }
                            }
                            DataTableBody {
                                for u in users().iter().cloned() {
                                    {
                                        let id = user_id(&u);
                                        let display = {
                                            let un = json_str(&u, "username");
                                            if un.is_empty() {
                                                json_str(&u, "name")
                                            } else {
                                                un
                                            }
                                        };
                                        let mail = json_str(&u, "email");
                                        let company = json_str(&u, "companyName");
                                        let user_role = {
                                            let r = json_str(&u, "role");
                                            if r.is_empty() { "editor".into() } else { r }
                                        };
                                        let active = user_active(&u);
                                        let sub = if company.is_empty() {
                                            mail.clone()
                                        } else {
                                            format!("{mail} · {company}")
                                        };
                                        let uid_role = id.clone();
                                        let uid_status = id.clone();
                                        let uid_edit = id.clone();
                                        let has_override = u.get("apiAccess").is_some()
                                            && !u.get("apiAccess").map(|x| x.is_null()).unwrap_or(true);
                                        let access_label = if has_override {
                                            "Custom".to_string()
                                        } else {
                                            "Role default".to_string()
                                        };
                                        let access_json = u
                                            .get("apiAccess")
                                            .cloned()
                                            .unwrap_or(serde_json::json!({}));
                                        rsx! {
                                            DataTableRow {
                                                key: "{id}",
                                                DataTableCell { class: Some("px-3 py-2".into()),
                                                    div { class: "font-medium", "{display}" }
                                                    div { class: "text-xs text-muted-foreground", "{sub}" }
                                                }
                                                DataTableCell { class: Some("px-3 py-2".into()),
                                                    RoleSelect {
                                                        class: Some("h-8 text-xs".into()),
                                                        value: user_role.clone(),
                                                        on_change: move |next| {
                                                            let uid = uid_role.clone();
                                                            spawn(async move {
                                                                let _ = crate::api::patch_json(
                                                                    &format!("/api/users/{uid}"),
                                                                    serde_json::json!({ "role": next }),
                                                                )
                                                                .await;
                                                                if let Ok(v) = crate::api::get_json("/api/users").await {
                                                                    users.set(
                                                                        v.get("users")
                                                                            .and_then(|x| x.as_array())
                                                                            .cloned()
                                                                            .unwrap_or_default(),
                                                                    );
                                                                }
                                                            });
                                                        },
                                                    }
                                                }
                                                DataTableCell { class: Some("px-3 py-2".into()),
                                                    div { class: "flex flex-col gap-1",
                                                        span { class: "text-xs text-muted-foreground", "{access_label}" }
                                                        Button {
                                                            class: Some("h-7 w-fit text-xs".into()),
                                                            variant: ButtonVariant::Outline,
                                                            onclick: move |_| {
                                                                editing_id.set(Some(uid_edit.clone()));
                                                                override_access.set(has_override);
                                                                access.set(if has_override {
                                                                    ApiAccessDraft::from_json(&access_json)
                                                                } else {
                                                                    ApiAccessDraft::default()
                                                                });
                                                            },
                                                            "Configure"
                                                        }
                                                    }
                                                }
                                                DataTableCell { class: Some("px-3 py-2".into()),
                                                    Switch {
                                                        size: Some("sm".into()),
                                                        checked: active,
                                                        on_checked_change: move |checked| {
                                                            let uid = uid_status.clone();
                                                            let status = if checked { "active" } else { "disabled" };
                                                            spawn(async move {
                                                                let _ = crate::api::patch_json(
                                                                    &format!("/api/users/{uid}"),
                                                                    serde_json::json!({ "status": status }),
                                                                )
                                                                .await;
                                                                if let Ok(v) = crate::api::get_json("/api/users").await {
                                                                    users.set(
                                                                        v.get("users")
                                                                            .and_then(|x| x.as_array())
                                                                            .cloned()
                                                                            .unwrap_or_default(),
                                                                    );
                                                                }
                                                            });
                                                        },
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
        }
    }
}
