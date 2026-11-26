use dioxus::prelude::*;

#[component]
pub fn Input(
    #[props(default)] class: Option<String>,
    #[props(default)] value: String,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] r#type: Option<String>,
    #[props(default)] disabled: bool,
    oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    let class = format!(
        "flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring {}",
        class.unwrap_or_default()
    );
    rsx! {
        input {
            r#type: r#type.unwrap_or_else(|| "text".into()),
            class: "{class}",
            value,
            placeholder: placeholder.unwrap_or_default(),
            disabled,
            oninput: move |e| {
                if let Some(h) = &oninput {
                    h.call(e);
                }
            },
        }
    }
}
