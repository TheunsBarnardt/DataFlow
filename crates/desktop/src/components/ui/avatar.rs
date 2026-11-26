use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarSize {
    Sm,
    #[default]
    Default,
    Lg,
}

impl AvatarSize {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Default => "default",
            Self::Lg => "lg",
        }
    }
}

#[component]
pub fn Avatar(
    #[props(default)] size: AvatarSize,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "group/avatar relative flex size-8 shrink-0 overflow-hidden rounded-full select-none after:absolute after:inset-0 after:rounded-full after:border after:border-border data-[size=lg]:size-10 data-[size=sm]:size-6",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            class: "{class}",
            "data-slot": "avatar",
            "data-size": "{size.as_str()}",
            {children}
        }
    }
}

#[component]
pub fn AvatarFallback(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "absolute inset-0 flex size-full items-center justify-center rounded-full bg-muted text-sm font-medium text-muted-foreground group-data-[size=sm]/avatar:text-xs",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { class: "{class}", {children} }
    }
}
