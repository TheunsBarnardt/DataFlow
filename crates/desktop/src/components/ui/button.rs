use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
}

fn variant_class(v: ButtonVariant) -> &'static str {
    match v {
        ButtonVariant::Default => "bg-primary text-primary-foreground hover:bg-primary/90",
        ButtonVariant::Destructive => "bg-destructive text-white hover:bg-destructive/90",
        ButtonVariant::Outline => "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground",
        ButtonVariant::Secondary => "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ButtonVariant::Ghost => "hover:bg-accent hover:text-accent-foreground",
        ButtonVariant::Link => "text-primary underline-offset-4 hover:underline",
    }
}

fn size_class(s: ButtonSize) -> &'static str {
    match s {
        ButtonSize::Default => "h-9 px-4 py-2",
        ButtonSize::Sm => "h-8 gap-1.5 rounded-md px-3",
        ButtonSize::Lg => "h-10 rounded-md px-6",
        ButtonSize::Icon => "size-9",
    }
}

#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] r#type: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let class = format!(
        "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 {} {} {}",
        variant_class(variant),
        size_class(size),
        class.unwrap_or_default()
    );
    rsx! {
        button {
            r#type: r#type.unwrap_or_else(|| "button".into()),
            class: "{class}",
            disabled,
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e);
                }
            },
            {children}
        }
    }
}
