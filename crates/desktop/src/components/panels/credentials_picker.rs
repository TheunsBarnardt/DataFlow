use dioxus::prelude::*;
use std::sync::RwLock;

use super::credentials::{credential_display_name, CredentialDialog};
use super::{json_id, json_string};

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
    let cached = read_cache().unwrap_or_default();
    let mut items = use_signal(|| cached.clone());
    let mut loading = use_signal(|| cached.is_empty());
    let mut load_error = use_signal(|| None::<String>);
    let mut reload_tick = use_signal(|| 0u32);
    let mut dialog_open = use_signal(|| false);

    use_effect(move || {
        let _ = reload_tick();
        // Warm cache hits skip the loading flash on NDV remounts.
        if let Some(cached) = read_cache() {
            if reload_tick() == 0 {
                items.set(cached);
                loading.set(false);
                return;
            }
        }
        spawn(async move {
            loading.set(true);
            load_error.set(None);
            match crate::api::get_json("/api/credentials").await {
                Ok(v) => {
                    let list = v
                        .get("credentials")
                        .and_then(|c| c.as_array())
                        .cloned()
                        .unwrap_or_default();
                    write_cache(list.clone());
                    items.set(list);
                }
                Err(e) => {
                    items.set(Vec::new());
                    load_error.set(Some(e));
                }
            }
            loading.set(false);
        });
    });

    let type_filter = credential_type.clone().unwrap_or_default();
    let all_items = items();
    let filtered: Vec<serde_json::Value> = all_items
        .iter()
        .filter(|c| credential_matches_type(c, &type_filter))
        .cloned()
        .collect();
    let options = if filtered.is_empty() {
        all_items
    } else {
        filtered
    };

    let type_label = credential_type
        .as_ref()
        .map(|t| credential_display_name(t))
        .unwrap_or_else(|| credential_type.clone().unwrap_or_else(|| "credential".into()));

    let waiting = loading() && items().is_empty();
    let placeholder = if waiting {
        "Loading credentials…".to_string()
    } else if let Some(err) = load_error() {
        format!("Failed to load ({err})")
    } else if options.is_empty() {
        "No credentials yet".to_string()
    } else {
        "Select credential…".to_string()
    };

    rsx! {
        select {
            id: id,
            class: "flex h-9 w-full rounded-md border border-input bg-background px-2 text-sm",
            value: "{value}",
            disabled: waiting,
            onchange: move |e| {
                let next = e.value();
                if next == CREATE_VALUE {
                    dialog_open.set(true);
                    return;
                }
                on_change.call(next);
            },
            option {
                value: "",
                "{placeholder}"
            }
            for c in options {
                {
                    let cid = json_id(&c);
                    let name = c.get("name").map(json_string).unwrap_or_else(|| cid.clone());
                    let ty = c.get("type").map(json_string).unwrap_or_default();
                    let label = if ty.is_empty() {
                        name.clone()
                    } else {
                        format!("{name} ({ty})")
                    };
                    rsx! {
                        option { key: "{cid}", value: "{cid}", "{label}" }
                    }
                }
            }
            option {
                disabled: true,
                value: "__sep__",
                "────────────"
            }
            option {
                value: CREATE_VALUE,
                class: "text-primary",
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
