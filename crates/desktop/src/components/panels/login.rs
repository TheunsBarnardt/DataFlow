use base64::Engine;
use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, Button, Card, CardContent, CardDescription, CardFooter, CardHeader,
    Input, Label,
};

fn logo_src() -> String {
    const BYTES: &[u8] = include_bytes!("../../../assets/dataflow-logo.png");
    let b64 = base64::engine::general_purpose::STANDARD.encode(BYTES);
    format!("data:image/png;base64,{b64}")
}

#[component]
pub fn LoginPage(on_logged_in: EventHandler<(String, String)>) -> Element {
    let mut email = use_signal(|| "admin".to_string());
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);
    let logo = use_memo(logo_src);

    rsx! {
        div {
            class: "relative flex h-full w-full min-h-0 items-center justify-center overflow-hidden px-4",
            style: "background:
                radial-gradient(ellipse 90% 55% at 50% -15%, rgba(77, 184, 233, 0.22), transparent 55%),
                radial-gradient(ellipse 55% 45% at 100% 100%, rgba(245, 130, 32, 0.12), transparent 50%),
                radial-gradient(ellipse 45% 40% at 0% 90%, rgba(29, 59, 134, 0.14), transparent 50%),
                var(--background);",
            Card { class: Some("relative z-10 w-full max-w-md border-border/60 shadow-lg".into()),
                CardHeader { class: Some("items-center gap-3 pb-2 text-center".into()),
                    img {
                        src: "{logo()}",
                        alt: "DATAFLOW — Workflow & API Automation",
                        class: "mx-auto h-auto w-full max-w-[280px] bg-transparent object-contain select-none",
                    }
                    CardDescription { class: Some("pt-1".into()),
                        "Sign in with your username and password"
                    }
                }
                form {
                    class: "flex flex-col gap-4",
                    onsubmit: move |_| {
                        busy.set(true);
                        let em = email().trim().to_string();
                        let pw = password();
                        spawn(async move {
                            let body = serde_json::json!({
                                "username": em,
                                "password": pw,
                            });
                            match crate::api::post_json("/api/v1/auth/login", body).await {
                                Ok(res) => {
                                    if let Some(token) = res.get("token").and_then(|v| v.as_str()) {
                                        crate::api::set_auth_token(Some(token.to_string()));
                                    }
                                    let role = res
                                        .get("role")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("admin")
                                        .to_string();
                                    err.set(None);
                                    on_logged_in.call((em, role));
                                }
                                Err(e) => err.set(Some(e)),
                            }
                            busy.set(false);
                        });
                    },
                    CardContent { class: Some("space-y-4".into()),
                        div { class: "flex flex-col gap-2",
                            Label {
                                r#for: Some("login-username".into()),
                                class: Some("text-muted-foreground".into()),
                                "Username"
                            }
                            Input {
                                id: Some("login-username".into()),
                                autocomplete: Some("username".into()),
                                value: email(),
                                required: true,
                                oninput: move |e: FormEvent| email.set(e.value()),
                            }
                        }
                        div { class: "flex flex-col gap-2",
                            Label {
                                r#for: Some("login-password".into()),
                                class: Some("text-muted-foreground".into()),
                                "Password"
                            }
                            Input {
                                id: Some("login-password".into()),
                                r#type: Some("password".into()),
                                autocomplete: Some("current-password".into()),
                                value: password(),
                                required: true,
                                oninput: move |e: FormEvent| password.set(e.value()),
                            }
                        }
                        if let Some(message) = err() {
                            Alert {
                                variant: crate::components::ui::AlertVariant::Destructive,
                                AlertDescription { "{message}" }
                            }
                        }
                    }
                    CardFooter {
                        Button {
                            class: Some(
                                "w-full !bg-[#1D3B86] text-white hover:!bg-[#162e6a]".into(),
                            ),
                            r#type: Some("submit".into()),
                            disabled: busy(),
                            if busy() { "Signing in…" } else { "Sign in" }
                        }
                    }
                }
            }
        }
    }
}
