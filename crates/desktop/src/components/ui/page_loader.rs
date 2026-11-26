use dioxus::prelude::*;

#[component]
pub fn PageLoader(
    #[props(default = "Loading…")] label: &'static str,
    #[props(default)] class: Option<&'static str>,
) -> Element {
    let extra = class.unwrap_or("");
    let root = format!(
        "flex h-full min-h-[12rem] flex-1 flex-col items-center justify-center gap-3 text-sm text-muted-foreground {extra}"
    );
    rsx! {
        div {
            class: "{root}",
            role: "status",
            "aria-live": "polite",
            div {
                class: "size-6 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground",
                "aria-hidden": "true",
            }
            span { "{label}" }
        }
    }
}
