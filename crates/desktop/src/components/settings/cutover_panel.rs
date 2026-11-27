use dioxus::prelude::*;

use crate::components::ui::{Alert, AlertDescription, Card, CardContent};

#[component]
pub fn CutoverPanel() -> Element {
    let mut phases = use_signal(Vec::<serde_json::Value>::new);
    let mut guidance = use_signal(String::new);
    let mut err = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            match crate::api::get_json("/api/platform/cutover").await {
                Ok(v) => {
                    phases.set(
                        v.get("phases")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    guidance.set(
                        v.get("guidance")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
        });
    });

    rsx! {
        div { class: "flex h-full min-h-0 flex-1 flex-col overflow-auto",
            div { class: "border-b px-8 py-5",
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Platform cutover" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "BotFlow replaces Wealth/Online/Dynamics as the API and backend. Shell clients should call /apis/{{slug}} only."
                }
            }

            div { class: "flex min-h-0 flex-1 flex-col gap-6 p-6",
                if let Some(message) = err() {
                    Alert {
                        AlertDescription { "{message}" }
                    }
                }

                if !guidance().is_empty() {
                    Card {
                        CardContent { class: Some("p-4 text-sm text-muted-foreground".into()),
                            "{guidance()}"
                        }
                    }
                }

                Card { class: Some("min-h-0 flex-1 overflow-hidden p-0 gap-0".into()),
                    CardContent { class: Some("divide-y overflow-y-auto p-0".into()),
                        for p in phases() {
                            {
                                let label = p.get("label").and_then(|x| x.as_str()).unwrap_or("");
                                let status = p.get("status").and_then(|x| x.as_str()).unwrap_or("");
                                let id = p.get("id").and_then(|x| x.as_str()).unwrap_or("");
                                rsx! {
                                    div { class: "flex items-center justify-between gap-3 px-4 py-3",
                                        div {
                                            div { class: "text-sm font-medium", "{label}" }
                                            code { class: "text-[11px] text-muted-foreground", "{id}" }
                                        }
                                        span { class: "text-xs text-muted-foreground", "{status}" }
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
