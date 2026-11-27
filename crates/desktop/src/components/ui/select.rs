//! Rust/UI-styled Select for form fields.
//! Uses a native `<select>` with rust-ui Input visual language so controlled
//! value / option lists work reliably in the desktop WebView.
//! Full portal Select: https://rust-ui.com/docs/components/select

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Select(
    #[props(default)] class: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default)] value: String,
    #[props(default)] disabled: bool,
    #[props(default)] required: bool,
    #[props(default)] title: Option<String>,
    onchange: Option<EventHandler<FormEvent>>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "border-input dark:bg-input/30 focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full min-w-0 rounded-md border bg-transparent px-2 text-sm shadow-xs outline-none transition-[color,box-shadow] focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        select {
            "data-slot": "select",
            class: "{class}",
            id: id,
            value,
            disabled,
            required,
            title: title,
            onchange: move |e| {
                if let Some(h) = &onchange {
                    h.call(e);
                }
            },
            {children}
        }
    }
}

#[component]
pub fn SelectOption(
    #[props(default)] value: String,
    #[props(default)] selected: bool,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    rsx! {
        option {
            value: "{value}",
            selected,
            disabled,
            {children}
        }
    }
}
