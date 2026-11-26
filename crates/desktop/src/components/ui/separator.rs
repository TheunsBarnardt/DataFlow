use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[component]
pub fn Separator(
    #[props(default)] class: Option<String>,
    #[props(default)] orientation: SeparatorOrientation,
) -> Element {
    let orientation_class = match orientation {
        SeparatorOrientation::Horizontal => "h-px w-full",
        SeparatorOrientation::Vertical => "h-full w-px",
    };
    let class = tw_merge!(
        "shrink-0 bg-border",
        orientation_class,
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            role: "separator",
            "aria-orientation": match orientation {
                SeparatorOrientation::Horizontal => "horizontal",
                SeparatorOrientation::Vertical => "vertical",
            },
            class: "{class}",
        }
    }
}
