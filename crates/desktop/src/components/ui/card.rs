use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CardSize {
    #[default]
    Default,
    Sm,
}

#[component]
pub fn Card(
    #[props(default)] class: Option<String>,
    #[props(default)] size: CardSize,
    children: Element,
) -> Element {
    let size_classes = match size {
        CardSize::Default => "py-6 gap-4",
        CardSize::Sm => "py-4 gap-3",
    };
    let data_size = match size {
        CardSize::Default => "default",
        CardSize::Sm => "sm",
    };
    let class = tw_merge!(
        "bg-card text-card-foreground flex flex-col rounded-xl border shadow-sm",
        size_classes,
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            "data-slot": "card",
            "data-size": "{data_size}",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn CardHeader(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "flex flex-col items-start gap-1.5 px-6 [[data-size=sm]_&]:px-4 [.border-b]:pb-6 has-data-[slot=card-action]:grid has-data-[slot=card-action]:grid-cols-[1fr_auto]",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { "data-slot": "card-header", class: "{class}", {children} }
    }
}

#[component]
pub fn CardTitle(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!("leading-none font-semibold", class.as_deref().unwrap_or(""));
    rsx! {
        h2 { "data-slot": "card-title", class: "{class}", {children} }
    }
}

#[component]
pub fn CardDescription(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        p { "data-slot": "card-description", class: "{class}", {children} }
    }
}

#[component]
pub fn CardContent(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "px-6 [[data-size=sm]_&]:px-4",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { "data-slot": "card-content", class: "{class}", {children} }
    }
}

#[component]
pub fn CardFooter(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "flex items-center gap-2 px-6 [[data-size=sm]_&]:px-4 [.border-t]:pt-6",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        footer { "data-slot": "card-footer", class: "{class}", {children} }
    }
}

#[component]
pub fn CardAction(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "self-start col-start-2 row-span-2 row-start-1 justify-self-end",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { "data-slot": "card-action", class: "{class}", {children} }
    }
}
