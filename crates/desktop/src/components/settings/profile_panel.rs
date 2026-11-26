use dioxus::prelude::*;

use crate::components::ui::{Button, Input, Label};

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

#[component]
pub fn ProfilePanel() -> Element {
    let mut user = use_signal(|| None::<serde_json::Value>);
    let mut name = use_signal(String::new);
    let mut company_name = use_signal(String::new);
    let mut err = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut loading = use_signal(|| true);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            match crate::api::get_json("/api/me").await {
                Ok(v) => {
                    if let Some(u) = v.get("user").cloned() {
                        name.set(json_str(&u, "name"));
                        company_name.set(json_str(&u, "companyName"));
                        user.set(Some(u));
                    }
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

    let save = move |_| {
        let next_name = name();
        let next_company = company_name();
        busy.set(true);
        spawn(async move {
            match crate::api::patch_json(
                "/api/me",
                serde_json::json!({
                    "name": next_name,
                    "companyName": next_company,
                }),
            )
            .await
            {
                Ok(v) => {
                    if let Some(u) = v.get("user").cloned() {
                        name.set(json_str(&u, "name"));
                        company_name.set(json_str(&u, "companyName"));
                        user.set(Some(u));
                    }
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            busy.set(false);
        });
    };

    if loading() && user().is_none() {
        return rsx! {
            div { class: "p-6 text-sm text-muted-foreground", "Loading profile…" }
        };
    }

    let u = user();
    let username = u
        .as_ref()
        .map(|x| json_str(x, "username"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".into());
    let email = u.as_ref().map(|x| json_str(x, "email")).unwrap_or_default();
    let role = u
        .as_ref()
        .map(|x| json_str(x, "role"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".into());

    rsx! {
        div { class: "mx-auto flex h-full max-w-lg flex-col gap-6 overflow-auto px-8 py-5",
            div {
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Profile" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Account details and preferences"
                }
            }

            if let Some(message) = err() {
                div { class: "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive",
                    "{message}"
                }
            }

            div { class: "space-y-3 rounded-lg border p-4",
                div { class: "grid gap-1 text-sm",
                    span { class: "text-muted-foreground", "Username" }
                    span { class: "font-medium", "{username}" }
                }
                div { class: "grid gap-1 text-sm",
                    span { class: "text-muted-foreground", "Email" }
                    span { class: "font-medium", "{email}" }
                }
                div { class: "grid gap-1 text-sm",
                    span { class: "text-muted-foreground", "Role" }
                    span { class: "font-medium", "{role}" }
                }
                div { class: "flex flex-col gap-1.5",
                    Label { class: Some("text-xs text-muted-foreground".into()), "Display name" }
                    Input {
                        value: name(),
                        oninput: move |e: FormEvent| name.set(e.value()),
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    Label { class: Some("text-xs text-muted-foreground".into()), "Company" }
                    Input {
                        value: company_name(),
                        placeholder: Some("Optional".into()),
                        oninput: move |e: FormEvent| company_name.set(e.value()),
                    }
                }
                Button {
                    disabled: busy(),
                    onclick: save,
                    "Save profile"
                }
            }
        }
    }
}