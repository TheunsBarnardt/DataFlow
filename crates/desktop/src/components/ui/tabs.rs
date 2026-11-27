//! Rust/UI Tabs — https://rust-ui.com/docs/components/tabs

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsVariant {
    #[default]
    Default,
    Line,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct TabsCtx {
    active: Signal<String>,
    on_value_change: Option<EventHandler<String>>,
}

#[derive(Clone, Copy)]
struct TabsListCtx {
    variant: TabsVariant,
}

#[component]
pub fn Tabs(
    #[props(default)] class: Option<String>,
    #[props(default)] default_value: Option<String>,
    /// Controlled value — when set, keeps the tabs in sync with parent state.
    #[props(default)]
    value: Option<String>,
    #[props(default)] on_value_change: Option<EventHandler<String>>,
    #[props(default)] orientation: TabsOrientation,
    children: Element,
) -> Element {
    let initial = value
        .clone()
        .or(default_value)
        .unwrap_or_default();
    let mut active = use_signal(|| initial.clone());
    if let Some(v) = value.as_ref() {
        if *active.peek() != *v {
            active.set(v.clone());
        }
    }
    provide_context(TabsCtx {
        active,
        on_value_change,
    });

    let is_horizontal = orientation == TabsOrientation::Horizontal;
    let class = tw_merge!(
        "group/tabs flex gap-2",
        if is_horizontal { "flex-col" } else { "flex-row" },
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div {
            "data-slot": "tabs",
            "data-orientation": if is_horizontal { "Horizontal" } else { "Vertical" },
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TabsList(
    #[props(default)] class: Option<String>,
    #[props(default)] variant: TabsVariant,
    children: Element,
) -> Element {
    provide_context(TabsListCtx { variant });

    let is_line = variant == TabsVariant::Line;
    let class = tw_merge!(
        "group/tabs-list inline-flex w-fit items-center justify-center rounded-lg p-[3px] text-muted-foreground",
        "group-data-[orientation=Horizontal]/tabs:h-8",
        "group-data-[orientation=Vertical]/tabs:h-fit group-data-[orientation=Vertical]/tabs:flex-col",
        if is_line {
            "gap-1 rounded-none bg-transparent p-0"
        } else {
            "bg-muted"
        },
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div {
            "data-slot": "tabs-list",
            "data-variant": if is_line { "Line" } else { "Default" },
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TabsTrigger(
    value: String,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let TabsCtx {
        mut active,
        on_value_change,
    } = use_context::<TabsCtx>();
    let list_ctx = use_context::<TabsListCtx>();
    let is_line = list_ctx.variant == TabsVariant::Line;
    let val = value.clone();
    let is_active = active() == val;

    let class = tw_merge!(
        "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-1.5 py-0.5 text-sm font-medium whitespace-nowrap transition-all cursor-pointer select-none",
        "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none",
        "disabled:pointer-events-none disabled:opacity-50",
        "group-data-[orientation=Vertical]/tabs:w-full group-data-[orientation=Vertical]/tabs:justify-start",
        "after:absolute after:bg-foreground after:transition-opacity",
        "group-data-[orientation=Horizontal]/tabs:after:inset-x-0 group-data-[orientation=Horizontal]/tabs:after:bottom-[-5px] group-data-[orientation=Horizontal]/tabs:after:h-0.5",
        "group-data-[orientation=Vertical]/tabs:after:inset-y-0 group-data-[orientation=Vertical]/tabs:after:-right-1 group-data-[orientation=Vertical]/tabs:after:w-0.5",
        if is_active {
            "text-foreground"
        } else {
            "text-foreground/60 hover:text-foreground"
        },
        if !is_line && is_active {
            "bg-background shadow-sm dark:border-input dark:bg-input/30"
        } else {
            ""
        },
        if is_line && is_active {
            "after:opacity-100"
        } else {
            "after:opacity-0"
        },
        class.as_deref().unwrap_or("")
    );

    rsx! {
        button {
            r#type: "button",
            "data-slot": "tabs-trigger",
            "data-state": if is_active { "Active" } else { "Inactive" },
            class: "{class}",
            onclick: move |_| {
                active.set(value.clone());
                if let Some(cb) = on_value_change.as_ref() {
                    cb.call(value.clone());
                }
            },
            {children}
        }
    }
}

#[component]
pub fn TabsContent(
    value: String,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let TabsCtx { active, .. } = use_context::<TabsCtx>();
    let is_active = active() == value;

    let class = tw_merge!(
        "flex-1 text-sm outline-none",
        "group-data-[orientation=Horizontal]/tabs:mt-2 group-data-[orientation=Vertical]/tabs:ml-4",
        if is_active { "" } else { "hidden" },
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div {
            "data-slot": "tabs-content",
            class: "{class}",
            {children}
        }
    }
}
