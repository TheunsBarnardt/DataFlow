//! CodeMirror 6 editor for Dioxus desktop.
//! Mounts from `onmounted` (not a self-cancelling use_effect). No textarea fallback.

use dioxus::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use crate::components::panels::{normalize_drag_expr, DF_DRAG_MIME};
use crate::format_code::{format_code, CodeLanguage};

fn lang_id(language: CodeLanguage, hint: Option<&str>) -> &'static str {
    if let Some(h) = hint {
        match h.to_ascii_lowercase().as_str() {
            "sql" | "tsql" => return "sql",
            "json" => return "json",
            "js" | "javascript" | "ts" | "typescript" => return "javascript",
            _ => {}
        }
    }
    match language {
        CodeLanguage::Sql => "sql",
        CodeLanguage::Json => "json",
        CodeLanguage::Js => "javascript",
        CodeLanguage::Text => "plaintext",
    }
}

fn sanitize_dom_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Insert text at the live CodeMirror cursor; returns the full document afterwards.
async fn insert_at_cursor(editor_id: &str, text: &str) -> Option<String> {
    let result = document::eval(&format!(
        r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot || !slot.insertAtCursor) return null;
  return slot.insertAtCursor({text});
}})();"#,
        id = serde_json::to_string(editor_id).unwrap_or_else(|_| "\"\"".into()),
        text = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into()),
    ))
    .await
    .ok()?;
    match result {
        Value::String(s) => Some(s),
        _ => None,
    }
}

async fn consume_drag_expr() -> Option<String> {
    let result = document::eval(
        r#"return (function(){
  if (window.__bfExprDnD && typeof window.__bfExprDnD.consume === 'function') {
    return window.__bfExprDnD.consume();
  }
  return null;
})();"#,
    )
    .await
    .ok()?;
    match result {
        Value::String(s) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

#[component]
pub fn CodeEditor(
    value: ReadSignal<String>,
    on_change: EventHandler<String>,
    #[props(default)] language: CodeLanguage,
    #[props(default = 160)] height_px: u32,
    /// Stretch to fill the parent flex/height container instead of a fixed px height.
    #[props(default)] fill: bool,
    #[props(default)] read_only: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] language_hint: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default = true)] show_format: bool,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
) -> Element {
    let editor_id = use_hook(|| {
        sanitize_dom_id(
            &id.clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("bf-cm-{}", Uuid::new_v4())),
        )
    });
    let host_id = editor_id.clone();
    let lang = lang_id(language, language_hint.as_deref());
    let can_format = show_format && language.supports_format() && !read_only;
    let ph = placeholder.clone().unwrap_or_default();

    let mut drag_over = use_signal(|| false);
    let mut mounted = use_signal(|| false);

    let field_key = editor_id.clone();
    use_effect(move || {
        let Some(mut bus) = expr_bus else {
            return;
        };
        let Some(expr) = bus() else {
            return;
        };
        let active = active_field.and_then(|a| a());
        if active.as_deref() != Some(field_key.as_str()) {
            return;
        }
        bus.set(None);
        let normalized = normalize_drag_expr(&expr);
        let editor_id = field_key.clone();
        spawn(async move {
            let full = insert_at_cursor(&editor_id, &normalized).await;
            on_change.call(full.unwrap_or(normalized));
        });
    });

    // Keep CM doc in sync after mount.
    use_effect({
        let editor_id = editor_id.clone();
        move || {
            let next = value();
            if !*mounted.peek() {
                return;
            }
            let editor_id = editor_id.clone();
            spawn(async move {
                let _ = document::eval(&format!(
                    r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot) return false;
  slot.setValue({value});
  try {{ slot.view.requestMeasure(); }} catch (_) {{}}
  return true;
}})();"#,
                    id = serde_json::to_string(&editor_id).unwrap_or_else(|_| "\"\"".into()),
                    value = serde_json::to_string(&next).unwrap_or_else(|_| "\"\"".into()),
                ))
                .await;
            });
        }
    });

    let eid = editor_id.clone();
    use_drop(move || {
        let _ = document::eval(&format!(
            r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot) return false;
  try {{ slot.destroy(); }} catch (_) {{}}
  delete window.__bfCm[{id}];
  return true;
}})();"#,
            id = serde_json::to_string(&eid).unwrap_or_else(|_| "\"\"".into()),
        ));
    });

    let height_style = if fill {
        "min-height:0".to_string()
    } else {
        format!("height:{height_px}px;min-height:{height_px}px")
    };
    let root_class = if fill {
        "flex h-full min-h-0 w-full flex-1 flex-col gap-1"
    } else {
        "flex w-full flex-col gap-1"
    };
    let host_class = if fill {
        if drag_over() {
            "flex min-h-0 w-full flex-1 overflow-hidden rounded-md border border-input bg-background ring-2 ring-[var(--brand)]"
        } else {
            "flex min-h-0 w-full flex-1 overflow-hidden rounded-md border border-input bg-background focus-within:ring-2 focus-within:ring-ring"
        }
    } else if drag_over() {
        "h-full min-h-0 w-full overflow-hidden rounded-md border border-input bg-background ring-2 ring-[var(--brand)]"
    } else {
        "h-full min-h-0 w-full overflow-hidden rounded-md border border-input bg-background focus-within:ring-2 focus-within:ring-ring"
    };

    rsx! {
        div { class: "{root_class}",
            if can_format {
                div { class: "flex shrink-0 justify-end",
                    button {
                        r#type: "button",
                        class: "rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-muted",
                        title: "Format",
                        onclick: {
                            let editor_id = editor_id.clone();
                            move |_| {
                                let editor_id = editor_id.clone();
                                let fallback = value();
                                spawn(async move {
                                    // Prefer live CodeMirror doc — signal can lag behind typing.
                                    let live = document::eval(&format!(
                                        r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot) return null;
  return slot.view.state.doc.toString();
}})();"#,
                                        id = serde_json::to_string(&editor_id)
                                            .unwrap_or_else(|_| "\"\"".into()),
                                    ))
                                    .await
                                    .ok()
                                    .and_then(|v| match v {
                                        Value::String(s) => Some(s),
                                        _ => None,
                                    })
                                    .unwrap_or(fallback);

                                    let result = format_code(&live, language);
                                    if !result.ok {
                                        return;
                                    }
                                    let formatted = result.text;
                                    let _ = document::eval(&format!(
                                        r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot) return false;
  slot.setValue({value});
  try {{ slot.view.focus(); }} catch (_) {{}}
  return true;
}})();"#,
                                        id = serde_json::to_string(&editor_id)
                                            .unwrap_or_else(|_| "\"\"".into()),
                                        value = serde_json::to_string(&formatted)
                                            .unwrap_or_else(|_| "\"\"".into()),
                                    ))
                                    .await;
                                    on_change.call(formatted);
                                });
                            }
                        },
                        "Format"
                    }
                }
            }
            div {
                class: "{host_class}",
                "data-bf-code-editor": "1",
                style: "{height_style}",
                onfocusin: {
                    let editor_id = editor_id.clone();
                    move |_| {
                        if let Some(mut active) = active_field {
                            active.set(Some(editor_id.clone()));
                        }
                    }
                },
                "data-bf-drop-expr": "1",
                "data-bf-editor-id": "{editor_id}",
                ondragover: move |e| {
                    if read_only { return; }
                    e.prevent_default();
                    drag_over.set(true);
                },
                ondragleave: move |_| drag_over.set(false),
                ondrop: {
                    let editor_id = editor_id.clone();
                    move |e| {
                        if read_only { return; }
                        e.prevent_default();
                        drag_over.set(false);
                        let dt = e.data().data_transfer();
                        let from_dt = dt
                            .get_data(DF_DRAG_MIME)
                            .or_else(|| dt.get_as_text())
                            .filter(|s| !s.trim().is_empty());
                        let editor_id = editor_id.clone();
                        spawn(async move {
                            let expr = match from_dt {
                                Some(s) => s,
                                None => match consume_drag_expr().await {
                                    Some(s) => s,
                                    None => return,
                                },
                            };
                            let normalized = normalize_drag_expr(&expr);
                            let full = insert_at_cursor(&editor_id, &normalized).await;
                            on_change.call(full.unwrap_or(normalized));
                        });
                    }
                },
                div {
                    id: "{host_id}",
                    class: if fill {
                        "bf-cm-host h-full min-h-0 w-full flex-1"
                    } else {
                        "bf-cm-host h-full min-h-0 w-full"
                    },
                    style: "{height_style}",
                    onmounted: {
                        let editor_id = editor_id.clone();
                        let ph = ph.clone();
                        let lang = lang.to_string();
                        move |_| {
                            if *mounted.peek() {
                                return;
                            }
                            mounted.set(true);

                            let editor_id = editor_id.clone();
                            let initial = value.peek().clone();
                            let lang = lang.clone();
                            let ph = ph.clone();
                            let read_only = read_only;

                            spawn(async move {
                                tracing::warn!(editor = %editor_id, "CodeEditor onmounted");

                                for _ in 0..80 {
                                    let api = document::eval(
                                        "return !!(window.bfCodeMirror && window.bfCodeMirror.create);",
                                    )
                                    .await;
                                    if matches!(api, Ok(Value::Bool(true))) {
                                        break;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                }

                                // Sync create; keep channel open for change events via dioxus.send.
                                let script = format!(
                                    r#"
const id = {id};
const el = document.getElementById(id);
if (!el) {{
  dioxus.send("__err:missing host");
}} else if (!window.bfCodeMirror || !window.bfCodeMirror.create) {{
  dioxus.send("__err:bfCodeMirror missing");
}} else {{
  window.__bfCm = window.__bfCm || {{}};
  if (window.__bfCm[id]) {{
    try {{ window.__bfCm[id].destroy(); }} catch (_) {{}}
    delete window.__bfCm[id];
  }}
  el.innerHTML = '';
  try {{
    window.bfCodeMirror.create(el, {{
      id,
      language: {lang},
      readOnly: {read_only},
      value: {initial},
      placeholder: {placeholder},
      onChange: function(v) {{ dioxus.send(v); }}
    }});
    dioxus.send("__ready__");
  }} catch (e) {{
    dioxus.send("__err:" + String((e && e.message) || e));
  }}
}}
await dioxus.recv();
"#,
                                    id = serde_json::to_string(&editor_id)
                                        .unwrap_or_else(|_| "\"\"".into()),
                                    lang = serde_json::to_string(&lang)
                                        .unwrap_or_else(|_| "\"plaintext\"".into()),
                                    read_only = if read_only { "true" } else { "false" },
                                    initial = serde_json::to_string(&initial)
                                        .unwrap_or_else(|_| "\"\"".into()),
                                    placeholder = serde_json::to_string(&ph)
                                        .unwrap_or_else(|_| "\"\"".into()),
                                );

                                let mut eval = document::eval(&script);
                                loop {
                                    match eval.recv::<String>().await {
                                        Ok(msg) if msg == "__ready__" => {
                                            tracing::warn!(
                                                editor = %editor_id,
                                                "CodeEditor mounted"
                                            );
                                            // Re-push latest value in case it changed during boot.
                                            let latest = value.peek().clone();
                                            let _ = document::eval(&format!(
                                                r#"return (function(){{
  const slot = window.__bfCm && window.__bfCm[{id}];
  if (!slot) return false;
  slot.setValue({value});
  try {{ slot.view.requestMeasure(); }} catch (_) {{}}
  return true;
}})();"#,
                                                id = serde_json::to_string(&editor_id)
                                                    .unwrap_or_else(|_| "\"\"".into()),
                                                value = serde_json::to_string(&latest)
                                                    .unwrap_or_else(|_| "\"\"".into()),
                                            ))
                                            .await;
                                        }
                                        Ok(msg) if msg.starts_with("__err:") => {
                                            tracing::warn!(
                                                editor = %editor_id,
                                                err = %&msg[6..],
                                                "CodeEditor create failed"
                                            );
                                            break;
                                        }
                                        Ok(text) => {
                                            on_change.call(text);
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                editor = %editor_id,
                                                ?e,
                                                "CodeEditor channel closed"
                                            );
                                            break;
                                        }
                                    }
                                }
                            });
                        }
                    },
                }
            }
        }
    }
}
