use dioxus::prelude::*;
use serde_json::Value;

use super::ndv_data_pane::{normalize_drag_expr, DF_DRAG_MIME};
use super::variable_aware_field::{VariableAwareInput, VariableAwareTextarea};

fn is_expression(value: &str) -> bool {
    let t = value.trim();
    t.starts_with("={{") || t.starts_with("{{")
}

fn to_expression(value: &str) -> String {
    if is_expression(value) {
        return value.to_string();
    }
    if value.is_empty() {
        return "={{  }}".into();
    }
    format!("={{ {} }}", serde_json::to_string(value).unwrap_or_else(|_| value.into()))
}

fn from_expression(value: &str) -> String {
    let t = value.trim();
    let Some(inner) = t
        .strip_prefix("={{")
        .and_then(|rest| rest.strip_suffix("}}"))
    else {
        return value.to_string();
    };
    let inner = inner.trim();
    if (inner.starts_with('"') && inner.ends_with('"'))
        || (inner.starts_with('\'') && inner.ends_with('\''))
    {
        if inner.starts_with('\'') {
            return inner[1..inner.len() - 1].to_string();
        }
        if let Ok(s) = serde_json::from_str::<String>(inner) {
            return s;
        }
    }
    value.to_string()
}

const SUGGESTIONS: &[&str] = &["$json.", "$input.item", "$vars.", "$('NodeName').item.", "$now"];

fn apply_expr_insert(mut mode: Signal<&'static str>, on_change: EventHandler<String>, expr: &str) {
    let normalized = normalize_drag_expr(expr);
    mode.set("expression");
    on_change.call(normalized);
}

#[component]
pub fn ExpressionField(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] multiline: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
) -> Element {
    let field_id = id.clone().unwrap_or_else(|| "expr-field".into());

    let mut mode = use_signal(|| {
        if is_expression(&value) {
            "expression"
        } else {
            "fixed"
        }
    });

    let ph_fixed = placeholder.clone().unwrap_or_else(|| "Type $ for expressions".into());
    let ph_expr = "={{ $json.field }} — or drop a field here".to_string();

    let mut drag_over = use_signal(|| false);

    let field_id_effect = field_id.clone();
    use_effect(move || {
        let Some(mut bus) = expr_bus else {
            return;
        };
        let Some(expr) = bus() else {
            return;
        };
        let active = active_field.and_then(|a| a());
        if active.as_deref() != Some(field_id_effect.as_str()) {
            return;
        }
        apply_expr_insert(mode, on_change, &expr);
        bus.set(None);
    });

    rsx! {
        div {
            class: if drag_over() {
                "flex flex-col gap-1 rounded-md ring-2 ring-[var(--brand)]"
            } else {
                "flex flex-col gap-1 rounded-md"
            },
            onfocusin: {
                let field_id = field_id.clone();
                move |_| {
                    if let Some(mut active) = active_field {
                        active.set(Some(field_id.clone()));
                    }
                }
            },
            "data-bf-drop-expr": "1",
            ondragover: move |e| {
                e.prevent_default();
                drag_over.set(true);
            },
            ondragleave: move |_| drag_over.set(false),
            ondrop: move |e| {
                e.prevent_default();
                drag_over.set(false);
                let dt = e.data().data_transfer();
                let from_dt = dt
                    .get_data(DF_DRAG_MIME)
                    .or_else(|| dt.get_as_text())
                    .filter(|s| !s.trim().is_empty());
                spawn(async move {
                    let expr = match from_dt {
                        Some(s) => s,
                        None => {
                            let result = document::eval(
                                r#"return (function(){
  if (window.__bfExprDnD && typeof window.__bfExprDnD.consume === 'function') {
    return window.__bfExprDnD.consume();
  }
  return null;
})();"#,
                            )
                            .await
                            .ok();
                            match result {
                                Some(Value::String(s)) if !s.trim().is_empty() => s,
                                _ => return,
                            }
                        }
                    };
                    apply_expr_insert(mode, on_change, &expr);
                });
            },
            div { class: "flex shrink-0 items-center gap-1",
                button {
                    r#type: "button",
                    class: if mode() == "fixed" {
                        "rounded px-1.5 py-0.5 text-[10px] font-medium bg-primary text-primary-foreground"
                    } else {
                        "rounded px-1.5 py-0.5 text-[10px] font-medium bg-muted text-muted-foreground"
                    },
                    onclick: {
                        let value = value.clone();
                        move |_| {
                            if mode() == "fixed" {
                                return;
                            }
                            on_change.call(from_expression(&value));
                            mode.set("fixed");
                        }
                    },
                    "Fixed"
                }
                button {
                    r#type: "button",
                    class: if mode() == "expression" {
                        "rounded px-1.5 py-0.5 text-[10px] font-medium bg-primary text-primary-foreground"
                    } else {
                        "rounded px-1.5 py-0.5 text-[10px] font-medium bg-muted text-muted-foreground"
                    },
                    onclick: {
                        let value = value.clone();
                        move |_| {
                            if mode() == "expression" {
                                return;
                            }
                            on_change.call(to_expression(&value));
                            mode.set("expression");
                        }
                    },
                    "Expression"
                }
            }
            if mode() == "expression" {
                div { class: "flex flex-wrap gap-0.5",
                    for s in SUGGESTIONS {
                        {
                            let s = (*s).to_string();
                            let value = value.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: "rounded border px-1 text-[9px] text-muted-foreground hover:bg-muted",
                                    title: "Insert {s}",
                                    onclick: move |_| {
                                        let base = value.trim();
                                        let next = if base.is_empty() {
                                            format!("={{ {s} }}")
                                        } else if base.contains("}}") {
                                            base.replace("}}", &format!("{s} }}"))
                                        } else {
                                            format!("={{ {s} }}")
                                        };
                                        on_change.call(next);
                                    },
                                    "{s}"
                                }
                            }
                        }
                    }
                }
            }
            if multiline {
                VariableAwareTextarea {
                    id,
                    value: value.clone(),
                    placeholder: if mode() == "expression" { ph_expr.clone() } else { ph_fixed.clone() },
                    on_change,
                }
            } else {
                VariableAwareInput {
                    id,
                    value: value.clone(),
                    placeholder: if mode() == "expression" { ph_expr } else { ph_fixed },
                    on_change,
                }
            }
        }
    }
}