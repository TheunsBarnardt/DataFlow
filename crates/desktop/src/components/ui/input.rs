use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Input(
    #[props(default)] class: Option<String>,
    #[props(default)] value: String,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] r#type: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default)] autocomplete: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] readonly: bool,
    #[props(default)] required: bool,
    oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    let class = tw_merge!(
        "text-foreground file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50",
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
        "read-only:bg-muted",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        input {
            r#type: r#type.unwrap_or_else(|| "text".into()),
            class: "{class}",
            id: id,
            autocomplete: autocomplete,
            value,
            placeholder: placeholder.unwrap_or_default(),
            disabled,
            readonly,
            required,
            oninput: move |e| {
                if let Some(h) = &oninput {
                    h.call(e);
                }
            },
        }
    }
}
