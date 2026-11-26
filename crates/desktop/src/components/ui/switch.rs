use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Switch(
    checked: bool,
    on_checked_change: EventHandler<bool>,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
    #[props(default)] size: Option<String>,
) -> Element {
    let sm = size.as_deref() == Some("sm");
    let track_size = if sm { "h-3.5 w-6" } else { "h-6 w-11" };
    let thumb_size = if sm { "size-3" } else { "size-5" };
    let thumb_pos = if checked {
        if sm {
            "translate-x-2.5"
        } else {
            "translate-x-5"
        }
    } else {
        "translate-x-0"
    };
    let track_bg = if checked {
        "bg-primary"
    } else {
        "bg-input dark:bg-input/80"
    };
    let class = tw_merge!(
        "peer inline-flex shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent shadow-xs transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
        track_size,
        track_bg,
        class.as_deref().unwrap_or("")
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
                class: "pointer-events-none block rounded-full bg-background shadow-lg ring-0 transition-transform {thumb_size} {thumb_pos}",
            }
        }
    }
}
