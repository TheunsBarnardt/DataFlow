use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonVariant, Card, CardContent, CardHeader,
    CardTitle, Empty, EmptyDescription, Input, Label,
};

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

#[component]
pub fn CompaniesPanel() -> Element {
    let mut companies = use_signal(Vec::<serde_json::Value>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut name = use_signal(String::new);
    let mut external_ref = use_signal(String::new);
    let mut creating = use_signal(|| false);

    let reload = move || {
        spawn(async move {
            loading.set(true);
            match crate::api::get_json("/api/companies").await {
                Ok(v) => {
                    companies.set(
                        v.get("companies")
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

    let create = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set(Some("Enter a company name".into()));
            return;
        }
        creating.set(true);
        let ext = external_ref();
        spawn(async move {
            let mut body = serde_json::json!({ "name": n });
            if !ext.trim().is_empty() {
                body["externalRef"] = serde_json::json!(ext.trim());
            }
            match crate::api::post_json("/api/companies", body).await {
                Ok(_) => {
                    name.set(String::new());
                    external_ref.set(String::new());
                    err.set(None);
                    if let Ok(v) = crate::api::get_json("/api/companies").await {
                        companies.set(
                            v.get("companies")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                }
                Err(e) => err.set(Some(e)),
            }
            creating.set(false);
        });
    };

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Companies" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Tenants that scope users, API keys, and access. External ref can map to a CRM company id when needed."
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 mt-2".into()),
                    AlertDescription { class: Some("text-destructive".into()), "{message}" }
                }
            }

            div { class: "grid min-h-0 flex-1 gap-6 p-6 lg:grid-cols-[minmax(280px,360px)_1fr]",
                Card { class: Some("self-start".into()),
                    CardHeader { class: Some("px-4 pt-4 pb-0".into()),
                        CardTitle { class: Some("text-sm".into()), "Create company" }
                    }
                    CardContent { class: Some("space-y-3 px-4 pb-4".into()),
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "Name" }
                            Input {
                                class: Some("h-9".into()),
                                value: name(),
                                placeholder: Some("Acme Advisors".into()),
                                oninput: move |e: FormEvent| name.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-1.5",
                            Label { class: Some("text-xs text-muted-foreground".into()), "External ref (optional)" }
                            Input {
                                class: Some("h-9".into()),
                                value: external_ref(),
                                placeholder: Some("dynamics-account-guid".into()),
                                oninput: move |e: FormEvent| external_ref.set(e.value()),
                            }
                        }
                        Button { disabled: creating(), onclick: create, "Create" }
                    }
                }

                Card { class: Some("flex min-h-0 min-w-0 flex-col overflow-hidden p-0 gap-0".into()),
                    div { class: "shrink-0 border-b bg-muted/40 px-4 py-3",
                        h2 { class: "text-sm font-semibold",
                            if loading() { "Loading…" } else { "{companies().len()} companies" }
                        }
                    }
                    if companies().is_empty() && !loading() {
                        Empty { class: Some("border-0 py-8".into()),
                            EmptyDescription { "No companies yet." }
                        }
                    } else {
                        div { class: "min-h-0 flex-1 divide-y overflow-y-auto",
                            for c in companies() {
                                {
                                    let id = json_str(&c, "id");
                                    let cname = json_str(&c, "name");
                                    let ext = json_str(&c, "externalRef");
                                    let status = json_str(&c, "status");
                                    let del_id = id.clone();
                                    rsx! {
                                        div { class: "flex flex-wrap items-center gap-3 px-4 py-3",
                                            div { class: "min-w-[12rem] flex-1",
                                                div { class: "text-sm font-medium", "{cname}" }
                                                code { class: "text-[11px] text-muted-foreground", "{id}" }
                                            }
                                            div { class: "text-xs text-muted-foreground",
                                                if !ext.is_empty() { "Ref {ext} · " }
                                                "{status}"
                                            }
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| {
                                                    let key = del_id.clone();
                                                    spawn(async move {
                                                        let _ = crate::api::delete_json(&format!("/api/companies/{key}")).await;
                                                        if let Ok(v) = crate::api::get_json("/api/companies").await {
                                                            companies.set(
                                                                v.get("companies")
                                                                    .and_then(|x| x.as_array())
                                                                    .cloned()
                                                                    .unwrap_or_default(),
                                                            );
                                                        }
                                                    });
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
    }
}
