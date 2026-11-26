use dioxus::prelude::*;
use tw_merge::tw_merge;

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
    let variant_class = match variant {
        BadgeVariant::Default => {
            "border-transparent shadow-xs bg-primary text-primary-foreground"
        }
        BadgeVariant::Secondary => {
            "border-transparent bg-secondary text-secondary-foreground"
        }
        BadgeVariant::Destructive => {
            "border-transparent bg-destructive/15 text-destructive"
        }
        BadgeVariant::Outline => "text-foreground",
        BadgeVariant::Success => "border-transparent bg-primary/15 text-primary",
    };
    let class = tw_merge!(
        "inline-flex w-fit items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2",
        variant_class,
        class.as_deref().unwrap_or("")
    );
    rsx! {
        span { class: "{class}", {children} }
    }
}
