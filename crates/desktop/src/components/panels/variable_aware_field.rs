use dioxus::prelude::*;

use crate::components::panels::{json_string, json_id};

#[derive(Clone, PartialEq)]
struct VarItem {
    key: String,
    description: String,
}

fn token_query(value: &str) -> Option<(usize, String)> {
    let bytes = value.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        let c = bytes[i - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            i -= 1;
            continue;
        }
        if c == '$' {
            let query = value[i..].to_string();
            return Some((i - 1, query));
        }
        break;
    }
    None
}

fn key_query_from_token(query: &str) -> String {
    let q = query.trim().trim_start_matches('$').to_lowercase();
    if q.is_empty() {
        return String::new();
    }
    if let Some(rest) = q.strip_prefix("vars.") {
        return rest.to_string();
    }
    if "vars".starts_with(&q) {
        return String::new();
    }
    q
}

fn filter_vars(items: &[VarItem], query: &str) -> Vec<VarItem> {
    let key_q = key_query_from_token(query);
    if key_q.is_empty() {
        return items.to_vec();
    }
    items
        .iter()
        .filter(|v| {
            v.key.to_lowercase().contains(&key_q)
                || v.description.to_lowercase().contains(&key_q)
        })
        .cloned()
        .collect()
}

fn insert_var(value: &str, start: usize, key: &str) -> String {
    let builtins: &[(&str, &str)] = &[
        ("json.", "$json."),
        ("json", "$json."),
        ("input.item", "$input.item"),
        ("input", "$input.item"),
        ("now", "$now"),
        ("today", "$today"),
    ];
    let insert = builtins
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or_else(|| {
            if key.starts_with('$') {
                key.to_string()
            } else if key.starts_with("vars.") {
                format!("$vars.{}", key.trim_start_matches("vars."))
            } else {
                format!("$vars.{key}")
            }
        });
    let before = &value[..start.min(value.len())];
    let rest = &value[start.min(value.len())..];
    let after_token = rest
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '$'))
        .map(|(i, _)| &rest[i..])
        .unwrap_or("");
    format!("{before}{insert}{after_token}")
}

#[component]
fn VariableAwareBase(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] id: Option<String>,
    multiline: bool,
) -> Element {
    let mut vars = use_signal(Vec::<VarItem>::new);
    let mut open = use_signal(|| false);
    let mut active = use_signal(|| 0usize);

    use_effect(move || {
        spawn(async move {
            if let Ok(v) = crate::api::get_json("/api/variables").await {
                let list = v
                    .get("variables")
                    .and_then(|x| x.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| VarItem {
                        key: item
                            .get("key")
                            .map(json_string)
                            .unwrap_or_else(|| json_id(&item)),
                        description: item
                            .get("description")
                            .map(json_string)
                            .unwrap_or_default(),
                    })
                    .filter(|v| !v.key.is_empty())
                    .collect();
                vars.set(list);
            }
        });
    });

    let token = token_query(&value);
    let filtered = if let Some((_, ref q)) = token {
        filter_vars(&vars(), q)
    } else {
        Vec::new()
    };
    let show = open() && token.is_some() && (!filtered.is_empty() || value.contains('$'));
    let value_for_focus = value.clone();

    let builtins = [
        ("json.", "Current item JSON"),
        ("input.item", "Input item"),
        ("now", "Current time"),
        ("today", "Today"),
    ];

    let suggestions: Vec<(String, String)> = {
        let mut out = Vec::new();
        if let Some((_, ref q)) = token {
            let qn = q.trim_start_matches('$').to_lowercase();
            for (k, desc) in builtins {
                if qn.is_empty() || k.starts_with(&qn) || k.contains(&qn) {
                    out.push((k.to_string(), desc.to_string()));
                }
            }
            for v in filtered.iter().take(12) {
                out.push((v.key.clone(), v.description.clone()));
            }
        }
        out
    };

    rsx! {
        div { class: "relative flex flex-col gap-1",
            if multiline {
                textarea {
                    id: id.clone(),
                    class: "min-h-[72px] w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    placeholder: placeholder.clone().unwrap_or_else(|| "Type $ for expressions".into()),
                    value: "{value}",
                    spellcheck: false,
                    oninput: move |e| {
                        let v = e.value();
                        open.set(token_query(&v).is_some());
                        active.set(0);
                        on_change.call(v);
                    },
                    onfocus: {
                        let value_for_focus = value_for_focus.clone();
                        move |_| {
                            if token_query(&value_for_focus).is_some() {
                                open.set(true);
                            }
                        }
                    },
                }
            } else {
                input {
                    id: id,
                    class: "flex h-9 w-full rounded-md border border-input bg-background px-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    placeholder: placeholder.unwrap_or_else(|| "Type $ for expressions".into()),
                    value: "{value}",
                    spellcheck: false,
                    oninput: move |e| {
                        let v = e.value();
                        open.set(token_query(&v).is_some());
                        active.set(0);
                        on_change.call(v);
                    },
                    onfocus: {
                        let value_for_focus = value_for_focus.clone();
                        move |_| {
                            if token_query(&value_for_focus).is_some() {
                                open.set(true);
                            }
                        }
                    },
                }
            }
            if show && !suggestions.is_empty() {
                div {
                    class: "absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
                    for (i, (key, desc)) in suggestions.iter().enumerate() {
                        {
                            let key = key.clone();
                            let desc = desc.clone();
                            let selected = active() == i;
                            rsx! {
                                button {
                                    key: "{key}-{i}",
                                    r#type: "button",
                                    class: if selected {
                                        "flex w-full flex-col items-start rounded-sm bg-accent px-2 py-1.5 text-left text-xs"
                                    } else {
                                        "flex w-full flex-col items-start rounded-sm px-2 py-1.5 text-left text-xs hover:bg-muted"
                                    },
                                    onclick: {
                                        let value = value.clone();
                                        let key = key.clone();
                                        move |_| {
                                            if let Some((start, _)) = token_query(&value) {
                                                on_change.call(insert_var(&value, start, &key));
                                            } else {
                                                on_change.call(format!("$vars.{key}"));
                                            }
                                            open.set(false);
                                        }
                                    },
                                    span { class: "font-mono font-medium", "${key}" }
                                    if !desc.is_empty() {
                                        span { class: "text-[10px] text-muted-foreground", "{desc}" }
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

#[component]
pub fn VariableAwareInput(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] id: Option<String>,
) -> Element {
    rsx! {
        VariableAwareBase {
            value,
            on_change,
            placeholder,
            id,
            multiline: false,
        }
    }
}

#[component]
pub fn VariableAwareTextarea(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] id: Option<String>,
) -> Element {
    rsx! {
        VariableAwareBase {
            value,
            on_change,
            placeholder,
            id,
            multiline: true,
        }
    }
}
