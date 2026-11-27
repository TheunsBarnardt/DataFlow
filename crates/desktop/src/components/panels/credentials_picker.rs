use dioxus::prelude::*;
use std::sync::RwLock;

use super::credentials::{credential_display_name, CredentialDialog};
use super::{json_id, json_string};
use crate::components::ui::{Select, SelectOption};

const CREATE_VALUE: &str = "__create__";

static CRED_CACHE: RwLock<Option<Vec<serde_json::Value>>> = RwLock::new(None);

fn credential_matches_type(c: &serde_json::Value, type_filter: &str) -> bool {
    if type_filter.is_empty() {
        return true;
    }
    let ty = c.get("type").map(json_string).unwrap_or_default();
    ty == type_filter
        || ty.to_lowercase().contains(&type_filter.to_lowercase())
        || type_filter.to_lowercase().contains(&ty.to_lowercase())
}

fn read_cache() -> Option<Vec<serde_json::Value>> {
    CRED_CACHE.read().ok().and_then(|g| g.clone())
}

fn write_cache(items: Vec<serde_json::Value>) {
    if let Ok(mut g) = CRED_CACHE.write() {
        *g = Some(items);
    }
}

pub fn invalidate_credentials_cache() {
    if let Ok(mut g) = CRED_CACHE.write() {
        *g = None;
    }
}

#[component]
pub fn CredentialsPicker(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] credential_type: Option<String>,
    #[props(default)] id: Option<String>,
) -> Element {
    let mut reload_tick = use_signal(|| 0u32);
    let mut dialog_open = use_signal(|| false);

    let creds = use_resource(move || {
        let tick = reload_tick();
        async move {
            // Prefer warm cache on first mount to avoid a loading flash.
            if tick == 0 {
                if let Some(cached) = read_cache() {
                    return Ok(cached);
                }
            }
            let v = crate::api::get_json("/api/credentials").await?;
            let list = v
                .get("credentials")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            write_cache(list.clone());
            Ok::<Vec<serde_json::Value>, String>(list)
        }
    });

    let type_filter = credential_type.clone().unwrap_or_default();
    let (all_items, loading, load_error) = match &*creds.read() {
        Some(Ok(list)) => (list.clone(), false, None),
        Some(Err(e)) => (Vec::new(), false, Some(e.clone())),
        None => (read_cache().unwrap_or_default(), true, None),
    };

    let filtered: Vec<serde_json::Value> = all_items
        .iter()
        .filter(|c| credential_matches_type(c, &type_filter))
        .cloned()
        .collect();
    let options = if filtered.is_empty() {
        all_items.clone()
    } else {
        filtered
    };

    let type_label = credential_type
        .as_ref()
        .map(|t| credential_display_name(t))
        .unwrap_or_else(|| credential_type.clone().unwrap_or_else(|| "credential".into()));

    let selected_missing = !value.is_empty()
        && !loading
        && load_error.is_none()
        && !options.iter().any(|c| json_id(c) == value);

    let waiting = loading && options.is_empty();
    let placeholder = if waiting {
        "Loading credentials…".to_string()
    } else if let Some(err) = &load_error {
        format!("Failed to load ({err})")
    } else if selected_missing {
        "Saved credential missing — create or reselect…".to_string()
    } else if options.is_empty() {
        "No credentials yet".to_string()
    } else {
        "Select credential…".to_string()
    };

    // WebView <select> ignores the select `value` attribute; mark options with `selected`.
    let show_placeholder = value.is_empty() || selected_missing;
    let select_key = format!("{}:{}:{}", value, options.len(), reload_tick());

    rsx! {
        Select {
            id: id,
            key: "{select_key}",
            value: value.clone(),
            disabled: waiting,
            onchange: move |e: FormEvent| {
                let next = e.value();
                if next == CREATE_VALUE {
                    dialog_open.set(true);
                    return;
                }
                on_change.call(next);
            },
            SelectOption {
                value: String::new(),
                selected: show_placeholder,
                "{placeholder}"
            }
            for c in options {
                {
                    let cid = json_id(&c);
                    let is_selected = !show_placeholder && cid == value;
                    let name = c.get("name").map(json_string).unwrap_or_else(|| cid.clone());
                    let ty = c.get("type").map(json_string).unwrap_or_default();
                    let label = if ty.is_empty() {
                        name.clone()
                    } else {
                        format!("{name} ({ty})")
                    };
                    rsx! {
                        SelectOption {
                            key: "{cid}",
                            value: cid.clone(),
                            selected: is_selected,
                            "{label}"
                        }
                    }
                }
            }
            SelectOption {
                disabled: true,
                value: "__sep__".to_string(),
                "────────────"
            }
            SelectOption {
                value: CREATE_VALUE.to_string(),
                "+ Create new {type_label}…"
            }
        }

        if dialog_open() {
            CredentialDialog {
                open: true,
                on_open_change: move |open| dialog_open.set(open),
                preferred_type: credential_type.clone(),
                on_created: move |cred| {
                    invalidate_credentials_cache();
                    reload_tick.with_mut(|t| *t += 1);
                    let cid = json_id(&cred);
                    if !cid.is_empty() {
                        on_change.call(cid);
                    }
                },
            }
        }
    }
}

pub fn is_credential_field(key: &str) -> bool {
    key == "credentialId"
        || key.ends_with("CredentialId")
        || key == "credentials"
        || key.contains("credential")
}
