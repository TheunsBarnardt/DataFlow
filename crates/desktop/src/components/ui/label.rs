use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Label(
    #[props(default)] class: Option<String>,
    #[props(default)] r#for: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "flex items-center gap-2 text-sm leading-none font-medium select-none peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        label {
            r#for: r#for.unwrap_or_default(),
            class: "{class}",
            {children}
        }
    }
}
