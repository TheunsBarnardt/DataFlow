use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
    Success,
}

#[component]
pub fn Badge(
    #[props(default)] variant: BadgeVariant,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let base = "inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-medium transition-colors";
    let v = match variant {
        BadgeVariant::Default => "border-transparent bg-primary text-primary-foreground",
        BadgeVariant::Secondary => "border-transparent bg-secondary text-secondary-foreground",
        BadgeVariant::Destructive => "border-transparent bg-destructive/15 text-destructive",
        BadgeVariant::Outline => "text-foreground",
        BadgeVariant::Success => "border-transparent bg-primary/15 text-primary",
    };
    let class = format!("{base} {v} {}", class.unwrap_or_default());
    rsx! {
        span { class: "{class}", {children} }
    }
}
