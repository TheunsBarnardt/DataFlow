use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, Button, Card, CardContent, CardDescription, CardFooter, CardHeader,
    CardTitle, Input, Label,
};

#[component]
pub fn LoginPage(on_logged_in: EventHandler<()>) -> Element {
    let mut email = use_signal(|| "admin".to_string());
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    rsx! {
        div {
            class: "relative flex min-h-full items-center justify-center overflow-hidden bg-background px-4",
            div {
                class: "pointer-events-none absolute inset-0 opacity-70",
                style: "background:
                    radial-gradient(ellipse 80% 50% at 50% -20%, color-mix(in oklch, var(--brand) 22%, transparent), transparent),
                    radial-gradient(ellipse 60% 40% at 100% 100%, color-mix(in oklch, var(--brand) 10%, transparent), transparent);",
            }
            Card { class: Some("relative z-10 w-full max-w-sm shadow-md".into()),
                CardHeader { class: Some("items-center text-center".into()),
                    div {
                        class: "mb-2 flex h-10 w-10 items-center justify-center rounded-lg text-sm font-bold text-white",
                        style: "background: var(--brand)",
                        "DF"
                    }
                    CardTitle { class: Some("text-xl tracking-tight".into()), "DataFlow" }
                    CardDescription { "Sign in with your username and password" }
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
                                Ok(_) => {
                                    err.set(None);
                                    on_logged_in.call(());
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
                            class: Some("w-full".into()),
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
