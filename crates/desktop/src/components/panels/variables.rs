use dioxus::prelude::*;

use crate::components::icons::Variable as VariableIcon;
use crate::components::panels::{json_id, json_string};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonSize, ButtonVariant, Card,
    ConfirmDeleteDialog, Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
    EmptyTitle, Input, PageLoader,
};

#[component]
pub fn VariablesPanel() -> Element {
    let mut items = use_signal(Vec::<serde_json::Value>::new);
    let mut query = use_signal(String::new);
    let mut err = use_signal(|| None::<String>);
    let mut editing_id = use_signal(|| None::<String>);
    let mut draft_key = use_signal(String::new);
    let mut draft_value = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut loading = use_signal(|| true);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);

    let reload = move || {
        spawn(async move {
            match crate::api::get_json("/api/variables").await {
                Ok(v) => {
                    items.set(
                        v.get("variables")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let q = query().trim().to_lowercase();
    let filtered: Vec<serde_json::Value> = items()
        .into_iter()
        .filter(|v| {
            if q.is_empty() {
                return true;
            }
            let key = v.get("key").map(json_string).unwrap_or_default();
            let val = v.get("value").map(json_string).unwrap_or_default();
            let desc = v.get("description").map(json_string).unwrap_or_default();
            format!("{key} {val} {desc}").to_lowercase().contains(&q)
        })
        .collect();

    let cancel_edit = move |_| {
        editing_id.set(None);
        draft_key.set(String::new());
        draft_value.set(String::new());
    };

    let save = move |_| {
        let key = draft_key().trim().to_string();
        if key.is_empty() {
            err.set(Some("Key is required".into()));
            return;
        }
        let value = draft_value();
        let mode = editing_id();
        busy.set(true);
        spawn(async move {
            let result = match mode.as_deref() {
                Some("new") => {
                    crate::api::post_json(
                        "/api/variables",
                        serde_json::json!({
                            "key": key,
                            "value": value,
                            "type": "string"
                        }),
                    )
                    .await
                }
                Some(id) => {
                    crate::api::put_json(
                        &format!("/api/variables/{id}"),
                        serde_json::json!({
                            "key": key,
                            "value": value,
                            "type": "string"
                        }),
                    )
                    .await
                }
                None => Ok(serde_json::Value::Null),
            };
            match result {
                Ok(_) => {
                    editing_id.set(None);
                    draft_key.set(String::new());
                    draft_value.set(String::new());
                    if let Ok(v) = crate::api::get_json("/api/variables").await {
                        items.set(
                            v.get("variables")
                                .and_then(|x| x.as_array())
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: "flex h-full flex-col",
            div { class: "flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5",
                div {
                    h1 { class: "text-2xl font-semibold tracking-tight", "Variables" }
                    p { class: "mt-1 text-sm text-muted-foreground",
                        "Project variables usable in expressions as "
                        code { class: "rounded bg-muted px-1.5 py-0.5 text-xs", "$vars.KEY" }
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    Input {
                        class: Some("w-56".into()),
                        placeholder: Some("Search variables...".into()),
                        value: query(),
                        oninput: move |e: FormEvent| query.set(e.value()),
                    }
                    Button {
                        disabled: editing_id().as_deref() == Some("new"),
                        onclick: move |_| {
                            editing_id.set(Some("new".into()));
                            draft_key.set(String::new());
                            draft_value.set(String::new());
                        },
                        "Add variable"
                    }
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("shrink-0 rounded-none border-x-0 border-t-0 border-b".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            if loading() {
                PageLoader { label: "Loading variables…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    Card { class: Some("overflow-hidden p-0 gap-0".into()),
                        div { class: "grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] gap-3 border-b bg-muted/40 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground",
                            div { "Key" }
                            div { "Value" }
                            div { "Usage syntax" }
                            div { class: "text-right", "Actions" }
                        }

                        if editing_id().as_deref() == Some("new") {
                            div { class: "grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] items-center gap-3 border-b px-4 py-3",
                                Input {
                                    placeholder: Some("Key".into()),
                                    value: draft_key(),
                                    oninput: move |e: FormEvent| draft_key.set(e.value()),
                                }
                                Input {
                                    placeholder: Some("Value".into()),
                                    value: draft_value(),
                                    oninput: move |e: FormEvent| draft_value.set(e.value()),
                                }
                                code { class: "truncate text-sm text-muted-foreground",
                                    {
                                        let key = draft_key();
                                        let key = key.trim();
                                        let shown = if key.is_empty() { "..." } else { key };
                                        format!("$vars.{shown}")
                                    }
                                }
                                div { class: "flex justify-end gap-2",
                                    Button {
                                        size: ButtonSize::Sm,
                                        variant: ButtonVariant::Ghost,
                                        onclick: cancel_edit,
                                        "Cancel"
                                    }
                                    Button {
                                        size: ButtonSize::Sm,
                                        disabled: busy(),
                                        onclick: save,
                                        "Save"
                                    }
                                }
                            }
                        }

                        for item in filtered.iter().cloned() {
                            {
                                let id = json_id(&item);
                                let key = item.get("key").map(json_string).unwrap_or_default();
                                let value = item.get("value").map(json_string).unwrap_or_default();
                                let is_editing = editing_id().as_deref() == Some(id.as_str());
                                let del_id = id.clone();
                                let del_key = key.clone();
                                let edit_id = id.clone();
                                let edit_key = key.clone();
                                let edit_value = value.clone();
                                rsx! {
                                    div {
                                        key: "{id}",
                                        class: "grid grid-cols-[1.2fr_1.4fr_1.2fr_auto] items-center gap-3 border-b px-4 py-3 last:border-b-0",
                                        if is_editing {
                                            Input {
                                                value: draft_key(),
                                                oninput: move |e: FormEvent| draft_key.set(e.value()),
                                            }
                                            Input {
                                                value: draft_value(),
                                                oninput: move |e: FormEvent| draft_value.set(e.value()),
                                            }
                                            code { class: "truncate text-sm text-muted-foreground",
                                                {
                                                    let key = draft_key();
                                                    let key = key.trim();
                                                    let shown = if key.is_empty() { "..." } else { key };
                                                    format!("$vars.{shown}")
                                                }
                                            }
                                            div { class: "flex justify-end gap-2",
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    variant: ButtonVariant::Ghost,
                                                    onclick: cancel_edit,
                                                    "Cancel"
                                                }
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    disabled: busy(),
                                                    onclick: save,
                                                    "Save"
                                                }
                                            }
                                        } else {
                                            div { class: "truncate text-sm font-medium", "{key}" }
                                            div { class: "truncate text-sm text-muted-foreground", "{value}" }
                                            code { class: "truncate text-sm text-muted-foreground", "$vars.{key}" }
                                            div { class: "flex justify-end gap-2",
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    variant: ButtonVariant::Outline,
                                                    onclick: move |_| {
                                                        editing_id.set(Some(edit_id.clone()));
                                                        draft_key.set(edit_key.clone());
                                                        draft_value.set(edit_value.clone());
                                                    },
                                                    "Edit"
                                                }
                                                Button {
                                                    size: ButtonSize::Sm,
                                                    variant: ButtonVariant::Ghost,
                                                    onclick: move |_| {
                                                        confirm_delete.set(Some((del_id.clone(), del_key.clone())));
                                                    },
                                                    "Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if filtered.is_empty() && editing_id().as_deref() != Some("new") {
                            Empty { class: Some("border-0 py-10".into()),
                                EmptyHeader {
                                    EmptyMedia { variant: EmptyMediaVariant::Icon,
                                        VariableIcon { class: Some("size-5".into()) }
                                    }
                                    EmptyTitle { class: Some("text-sm".into()), "No variables yet" }
                                    EmptyDescription { "Click Add variable to create one." }
                                }
                            }
                        }
                    }
                }
            }
            }

            if let Some((del_id, del_key)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let description = format!(
                        "This will permanently remove $vars.{del_key}. This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete variable?".to_string(),
                            description,
                            busy: busy(),
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                let check_id = del_id_confirm.clone();
                                let path = format!("/api/variables/{check_id}");
                                busy.set(true);
                                spawn(async move {
                                    match crate::api::delete_json(&path).await {
                                        Ok(_) => {
                                            if editing_id().as_deref() == Some(check_id.as_str()) {
                                                editing_id.set(None);
                                            }
                                            confirm_delete.set(None);
                                            if let Ok(v) = crate::api::get_json("/api/variables").await {
                                                items.set(
                                                    v.get("variables")
                                                        .and_then(|x| x.as_array())
                                                        .cloned()
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            err.set(None);
                                        }
                                        Err(e) => err.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}
