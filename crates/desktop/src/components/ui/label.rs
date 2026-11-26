use dioxus::prelude::*;

#[component]
pub fn Label(
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = format!(
        "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 {}",
        class.unwrap_or_default()
    );
    rsx! {
        label { class: "{class}", {children} }
    }
}
