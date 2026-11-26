//! Port of web/src/components/WebhookListenPanel.tsx — test/production URLs and listen-for-sample.

use dioxus::prelude::*;
use serde_json::{json, Value};

use crate::components::icons::{Copy, RefreshCw};
use crate::components::ui::{Button, ButtonSize, ButtonVariant};

#[derive(Clone, PartialEq, Debug)]
pub struct WebhookSample {
    pub headers: Value,
    pub query: Value,
    pub body: Value,
    pub method: String,
    pub path: String,
    pub captured_at: String,
}

pub fn webhook_sample_from_value(v: &Value) -> Option<WebhookSample> {
    if !v.is_object() {
        return None;
    }
    Some(WebhookSample {
        headers: v
            .get("headers")
            .cloned()
            .unwrap_or_else(|| json!({})),
        query: v.get("query").cloned().unwrap_or(Value::Null),
        body: v.get("body").cloned().unwrap_or(Value::Null),
        method: v
            .get("method")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        path: v
            .get("path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        captured_at: v
            .get("capturedAt")
            .or_else(|| v.get("captured_at"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn public_origin() -> String {
    crate::api::base_url().trim_end_matches('/').to_string()
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn sample_preview_json(sample: &WebhookSample) -> String {
    let v = json!({
        "method": sample.method,
        "headers": sample.headers,
        "query": sample.query,
        "body": sample.body,
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

fn escape_js_template(s: &str) -> String {
    s.replace('\\', "\\\\").replace('`', "\\`")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UrlMode {
    Test,
    Production,
}

#[component]
pub fn WebhookListenPanel(
    workflow_id: String,
    path: String,
    method: String,
    workflow_enabled: bool,
    sample: Option<WebhookSample>,
    on_sample: EventHandler<Option<WebhookSample>>,
    #[props(default)] on_toast: Option<EventHandler<String>>,
) -> Element {
    let mut mode = use_signal(|| UrlMode::Test);
    let mut listening = use_signal(|| false);
    let mut expires_at = use_signal(|| None::<u64>);
    let mut now_ms = use_signal(now_millis);
    let mut copied = use_signal(|| false);

    let path_for_empty = path.clone();
    let path_for_url = path.clone();
    let absolute_url = use_memo(move || {
        let p = path_for_url.trim();
        if p.is_empty() {
            return String::new();
        }
        let suffix = match mode() {
            UrlMode::Test => format!("/hooks-test/{p}"),
            UrlMode::Production => format!("/hooks/{p}"),
        };
        format!("{}{}", public_origin(), suffix)
    });

    let remaining_sec = use_memo(move || {
        let _ = now_ms();
        if !listening() {
            return 0u64;
        }
        let Some(exp) = expires_at() else {
            return 0;
        };
        let left = exp.saturating_sub(now_millis());
        (left + 999) / 1000
    });

    use_effect(move || {
        if !listening() || expires_at().is_none() {
            return;
        }
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if !listening() {
                    break;
                }
                now_ms.set(now_millis());
            }
        });
    });

    let path_empty = path_for_empty.trim().is_empty();
    let url = absolute_url();
    let preview = sample.as_ref().map(sample_preview_json).unwrap_or_default();

    rsx! {
        div { class: "mb-3 flex flex-col gap-3 rounded-lg border bg-muted/20 p-3",
            div { class: "flex items-center gap-1 rounded-md bg-muted/50 p-0.5",
                button {
                    class: if mode() == UrlMode::Test {
                        "flex-1 rounded-md px-2 py-1.5 text-xs font-medium bg-card shadow-sm"
                    } else {
                        "flex-1 rounded-md px-2 py-1.5 text-xs font-medium text-muted-foreground"
                    },
                    onclick: move |_| mode.set(UrlMode::Test),
                    "Test URL"
                }
                button {
                    class: if mode() == UrlMode::Production {
                        "flex-1 rounded-md px-2 py-1.5 text-xs font-medium bg-card shadow-sm"
                    } else {
                        "flex-1 rounded-md px-2 py-1.5 text-xs font-medium text-muted-foreground"
                    },
                    onclick: move |_| mode.set(UrlMode::Production),
                    "Production URL"
                }
            }

            div { class: "flex items-stretch gap-1",
                code { class: "min-w-0 flex-1 truncate rounded-md border bg-background px-2 py-1.5 font-mono text-[11px]",
                    if url.is_empty() {
                        "Save path to generate URL"
                    } else {
                        "{url}"
                    }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Icon,
                    class: Some("h-8 w-8 shrink-0".into()),
                    disabled: path_empty,
                    onclick: {
                        let url_copy = url.clone();
                        let toast = on_toast;
                        move |_| {
                            if url_copy.is_empty() {
                                return;
                            }
                            let escaped = escape_js_template(&url_copy);
                            let _ = document::eval(&format!(
                                "navigator.clipboard.writeText(`{escaped}`).catch(()=>{{}})"
                            ));
                            copied.set(true);
                            if let Some(t) = toast {
                                t.call("URL copied".into());
                            }
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                                copied.set(false);
                            });
                        }
                    },
                    if copied() {
                        span { class: "text-xs font-semibold", "✓" }
                    } else {
                        Copy { class: Some("h-3.5 w-3.5".into()) }
                    }
                }
            }

            if mode() == UrlMode::Production && !workflow_enabled {
                p { class: "text-[11px] text-amber-600 dark:text-amber-400",
                    "Enable the workflow to accept production webhook calls."
                }
            }

            if mode() == UrlMode::Test {
                Button {
                    size: ButtonSize::Sm,
                    variant: if listening() {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Default
                    },
                    disabled: path_empty || listening(),
                    class: Some("w-full".into()),
                    onclick: {
                        let wf_id = workflow_id.clone();
                        let path = path.clone();
                        let method = method.clone();
                        let toast = on_toast;
                        move |_| {
                            let path_trimmed = path.trim().to_string();
                            if path_trimmed.is_empty() {
                                if let Some(t) = toast {
                                    t.call("Set a webhook path before listening".into());
                                }
                                return;
                            }
                            listening.set(true);
                            on_sample.call(None);
                            expires_at.set(Some(now_millis() + 120_000));
                            if let Some(t) = toast {
                                t.call("Listening for test event — send a request to the Test URL".into());
                            }
                            let method = method.clone();
                            let wf_id = wf_id.clone();
                            spawn(async move {
                                let body = json!({
                                    "workflowId": wf_id,
                                    "path": path_trimmed,
                                    "method": method,
                                    "wait": true,
                                });
                                let result =
                                    crate::api::post_json("/api/webhooks/listen", body).await;
                                listening.set(false);
                                expires_at.set(None);
                                match result {
                                    Ok(res) => {
                                        if let Some(event) = res.get("event") {
                                            if let Some(sample) = webhook_sample_from_value(event)
                                            {
                                                on_sample.call(Some(sample));
                                                if let Some(t) = toast {
                                                    t.call("Test event received".into());
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        if let Some(t) = toast {
                                            t.call(e);
                                        }
                                    }
                                }
                            });
                        }
                    },
                    if listening() {
                        RefreshCw { class: Some("h-3.5 w-3.5 animate-spin".into()) }
                        span { "Listening… {remaining_sec}s" }
                    } else {
                        span { class: "text-xs", "◉" }
                        "Listen for test event"
                    }
                }
            } else {
                p { class: "text-[11px] text-muted-foreground",
                    "Production calls run the workflow and appear under Executions."
                }
            }

            if let Some(ref s) = sample {
                div { class: "rounded-md border bg-background p-2",
                    div { class: "mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground",
                        "Last test event"
                    }
                    pre { class: "max-h-40 overflow-auto font-mono text-[10px] leading-relaxed whitespace-pre-wrap break-all",
                        "{preview}"
                    }
                    span { class: "sr-only", "path: {s.path}" }
                }
            }
        }
    }
}