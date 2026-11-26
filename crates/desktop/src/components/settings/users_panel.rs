use dioxus::prelude::*;

use crate::components::ui::{Button, Input, Switch};
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
    let base = class.unwrap_or_default();
    rsx! {
        select {
            class: "flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring {base}",
            value: "{value}",
            onchange: move |e| on_change.call(e.value()),
            for role in DATAFLOW_ROLES.iter() {
                option { value: "{role}", "{role}" }
            }
        }
    }
}

#[component]
pub fn UsersPanel() -> Element {
    let mut users = use_signal(Vec::<serde_json::Value>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let mut email = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut role = use_signal(|| "editor".to_string());
    let mut company_name = use_signal(String::new);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            match crate::api::get_json("/api/users").await {
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
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let create_user = move |_| {
        let body = serde_json::json!({
            "email": email(),
            "username": username(),
            "name": name(),
            "password": password(),
            "role": role(),
            "companyName": company_name(),
        });
        busy.set(true);
        spawn(async move {
            match crate::api::post_json("/api/users", body).await {
                Ok(_) => {
                    email.set(String::new());
                    username.set(String::new());
                    name.set(String::new());
                    password.set(String::new());
                    company_name.set(String::new());
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

    rsx! {
        div { class: "flex h-full flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Users" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Create accounts and manage roles (admin only)"
                }
            }

            if let Some(message) = err() {
                div { class: "border-b bg-destructive/10 px-8 py-2 text-sm text-destructive", "{message}" }
            }

            div { class: "grid gap-6 p-6 lg:grid-cols-[minmax(280px,360px)_1fr]",
                div { class: "space-y-3 rounded-lg border p-4",
                    h2 { class: "text-sm font-semibold", "Create user" }
                    Input {
                        placeholder: Some("Display name".into()),
                        value: name(),
                        oninput: move |e: FormEvent| name.set(e.value()),
                    }
                    Input {
                        placeholder: Some("Username".into()),
                        value: username(),
                        oninput: move |e: FormEvent| username.set(e.value()),
                    }
                    Input {
                        placeholder: Some("Email".into()),
                        r#type: Some("email".into()),
                        value: email(),
                        oninput: move |e: FormEvent| email.set(e.value()),
                    }
                    Input {
                        placeholder: Some("Password".into()),
                        r#type: Some("password".into()),
                        value: password(),
                        oninput: move |e: FormEvent| password.set(e.value()),
                    }
                    Input {
                        placeholder: Some("Company (optional)".into()),
                        value: company_name(),
                        oninput: move |e: FormEvent| company_name.set(e.value()),
                    }
                    RoleSelect {
                        value: role(),
                        class: None,
                        on_change: move |r| role.set(r),
                    }
                    Button {
                        disabled: busy(),
                        onclick: create_user,
                        "Create"
                    }
                }

                div { class: "min-w-0 overflow-auto rounded-lg border",
                    if loading() {
                        div { class: "p-4 text-sm text-muted-foreground", "Loading…" }
                    } else if users().is_empty() {
                        div { class: "p-4 text-sm text-muted-foreground", "No users" }
                    } else {
                        table { class: "w-full text-left text-sm",
                            thead { class: "border-b bg-muted/40 text-xs text-muted-foreground",
                                tr {
                                    th { class: "px-3 py-2 font-medium", "User" }
                                    th { class: "px-3 py-2 font-medium", "Role" }
                                    th { class: "px-3 py-2 font-medium", "Active" }
                                }
                            }
                            tbody {
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
                                        rsx! {
                                            tr { class: "border-b last:border-0",
                                                td { class: "px-3 py-2",
                                                    div { class: "font-medium", "{display}" }
                                                    div { class: "text-xs text-muted-foreground", "{sub}" }
                                                }
                                                td { class: "px-3 py-2",
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
                                                td { class: "px-3 py-2",
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