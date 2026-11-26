use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Skeleton(#[props(default)] class: Option<String>) -> Element {
    let class = tw_merge!(
        "animate-pulse rounded-md bg-muted",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { class: "{class}", "aria-hidden": "true" }
    }
}
