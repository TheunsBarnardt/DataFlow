use dioxus::prelude::*;

#[component]
pub fn LoginPage(on_logged_in: EventHandler<()>) -> Element {
    let mut email = use_signal(|| "admin".to_string());
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    rsx! {
        div { class: "flex min-h-full items-center justify-center bg-background px-4",
            div { class: "w-full max-w-sm space-y-6",
                div { class: "space-y-1 text-center",
                    h1 { class: "text-2xl font-semibold tracking-tight", "DataFlow" }
                    p { class: "text-sm text-muted-foreground",
                        "Sign in with your username and password"
                    }
                }
                form {
                    class: "space-y-3 rounded-lg border p-5",
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
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs text-muted-foreground", r#for: "login-username", "Username" }
                        input {
                            id: "login-username",
                            class: "flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                            autocomplete: "username",
                            value: "{email()}",
                            oninput: move |e| email.set(e.value()),
                            required: true,
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs text-muted-foreground", "Password" }
                        input {
                            class: "flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                            r#type: "password",
                            autocomplete: "current-password",
                            value: "{password()}",
                            oninput: move |e| password.set(e.value()),
                            required: true,
                        }
                    }
                    if let Some(message) = err() {
                        div { class: "text-sm text-destructive", "{message}" }
                    }
                    button {
                        class: "inline-flex h-9 w-full items-center justify-center rounded-md bg-primary text-sm font-medium text-primary-foreground disabled:opacity-50",
                        r#type: "submit",
                        disabled: busy(),
                        if busy() { "Signing in…" } else { "Sign in" }
                    }
                }
            }
        }
    }
}