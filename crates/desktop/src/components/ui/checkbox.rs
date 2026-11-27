//! Rust/UI Checkbox — https://rust-ui.com/docs/components/checkbox

use dioxus::prelude::*;
use tw_merge::tw_merge;

use crate::components::icons::Check;

#[component]
pub fn Checkbox(
    #[props(default)] class: Option<String>,
    #[props(default)] checked: bool,
    #[props(default)] disabled: bool,
    #[props(default)] title: Option<String>,
    #[props(default = "Checkbox".to_string())] aria_label: String,
    on_checked_change: Option<EventHandler<bool>>,
) -> Element {
    let checked_state = if checked { "checked" } else { "unchecked" };
    let class = tw_merge!(
        "peer border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive size-4 shrink-0 rounded-[4px] border shadow-xs transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        button {
            "data-slot": "checkbox",
            "data-state": checked_state,
            r#type: "button",
            role: "checkbox",
            class: "{class}",
            "aria-checked": "{checked}",
            "aria-label": "{aria_label}",
            title: title,
            disabled,
            onclick: move |_| {
                if disabled {
                    return;
                }
                if let Some(h) = &on_checked_change {
                    h.call(!checked);
                }
            },
            span {
                "data-slot": "checkbox-indicator",
                class: "flex items-center justify-center text-current transition-none",
                if checked {
                    Check { class: Some("size-3.5".into()) }
                }
            }
        }
    }
}
