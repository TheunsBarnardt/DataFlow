use dioxus::prelude::*;

#[component]
pub fn Switch(
    checked: bool,
    on_checked_change: EventHandler<bool>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] size: Option<String>,
) -> Element {
    let sm = size.as_deref() == Some("sm");
    let track_size = if sm { "h-3.5 w-6" } else { "h-[1.15rem] w-8" };
    let thumb_size = if sm { "size-3" } else { "size-4" };
    let thumb_pos = if checked {
        if sm {
            "translate-x-2.5"
        } else {
            "translate-x-3.5"
        }
    } else {
        "translate-x-0"
    };
    let track_bg = if checked {
        "bg-primary"
    } else {
        "bg-input dark:bg-input/80"
    };
    let class = format!(
        "peer inline-flex shrink-0 cursor-pointer items-center rounded-full border border-transparent shadow-xs transition-all outline-none disabled:cursor-not-allowed disabled:opacity-50 {track_size} {track_bg} {}",
        class.unwrap_or_default()
    );
    rsx! {
        button {
            r#type: "button",
            role: "switch",
            "aria-checked": checked,
            class: "{class}",
            disabled,
            onclick: move |_| {
                if !disabled {
                    on_checked_change.call(!checked);
                }
            },
            span {
                class: "pointer-events-none block rounded-full bg-background shadow ring-0 transition-transform {thumb_size} {thumb_pos}",
            }
        }
    }
}
