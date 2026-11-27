//! Rust/UI Dialog — https://rust-ui.com/docs/components/dialog
//! Controlled overlay (open when mounted). Matches project modal patterns.

use dioxus::prelude::*;
use tw_merge::tw_merge;

use crate::components::icons::X;

#[component]
pub fn Dialog(
    #[props(default)] class: Option<String>,
    #[props(default)] on_close: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4 backdrop-blur-[1px]",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            class: "{merged}",
            "data-name": "__Dialog",
            onclick: move |_| {
                if let Some(h) = &on_close {
                    h.call(());
                }
            },
            {children}
        }
    }
}

#[component]
pub fn DialogContent(
    #[props(default)] class: Option<String>,
    #[props(default = true)] show_close_button: bool,
    #[props(default)] on_close: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "relative flex max-h-[min(85vh,720px)] w-full max-w-lg flex-col overflow-hidden rounded-2xl border bg-background p-6 shadow-lg",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            "data-name": "DialogContent",
            class: "{merged}",
            role: "dialog",
            "aria-modal": "true",
            onclick: move |e| e.stop_propagation(),
            if show_close_button {
                button {
                    r#type: "button",
                    class: "absolute top-4 right-4 rounded-sm p-1 opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 [&_svg:not([class*='size-'])]:size-4",
                    "aria-label": "Close dialog",
                    onclick: move |_| {
                        if let Some(h) = &on_close {
                            h.call(());
                        }
                    },
                    X { class: Some("size-4".into()) }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn DialogHeader(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "flex flex-col gap-2 text-center sm:text-left",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { class: "{merged}", {children} }
    }
}

#[component]
pub fn DialogTitle(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!("text-lg leading-none font-semibold", class.as_deref().unwrap_or(""));
    rsx! {
        h3 { class: "{merged}", {children} }
    }
}

#[component]
pub fn DialogDescription(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!("text-muted-foreground text-sm", class.as_deref().unwrap_or(""));
    rsx! {
        p { class: "{merged}", {children} }
    }
}

#[component]
pub fn DialogBody(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!("flex flex-col gap-4", class.as_deref().unwrap_or(""));
    rsx! {
        div { class: "{merged}", {children} }
    }
}

#[component]
pub fn DialogFooter(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        footer { class: "{merged}", {children} }
    }
}

#[component]
pub fn DialogClose(
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] on_close: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "inline-flex h-9 items-center justify-center gap-2 whitespace-nowrap rounded-md border bg-background px-4 py-2 text-sm font-medium shadow-xs hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        button {
            r#type: "button",
            class: "{merged}",
            disabled,
            "aria-label": "Close dialog",
            onclick: move |_| {
                if let Some(h) = &on_close {
                    h.call(());
                }
            },
            {children}
        }
    }
}

#[component]
pub fn DialogAction(
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "inline-flex h-9 items-center justify-center gap-2 whitespace-nowrap rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-xs hover:bg-primary/90 disabled:pointer-events-none disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        button {
            r#type: "button",
            class: "{merged}",
            disabled,
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e);
                }
            },
            {children}
        }
    }
}
