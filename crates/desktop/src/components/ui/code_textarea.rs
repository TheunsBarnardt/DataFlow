use dioxus::prelude::*;

use crate::format_code::{format_code, CodeLanguage};

#[component]
pub fn CodeTextarea(
    value: String,
    on_change: EventHandler<String>,
    #[props(default)] placeholder: Option<String>,
    #[props(default = 120)] min_height_px: u32,
    #[props(default)] language: CodeLanguage,
    #[props(default)] id: Option<String>,
) -> Element {
    let min_h = format!("min-height: {min_height_px}px");
    let show_format = language.supports_format();

    let format_src = value.clone();
    rsx! {
        div { class: "flex flex-col gap-1",
            if show_format {
                div { class: "flex justify-end",
                    button {
                        r#type: "button",
                        class: "rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-muted",
                        title: "Format",
                        onclick: move |_| {
                            let result = format_code(&format_src, language);
                            if result.ok {
                                on_change.call(result.text);
                            }
                        },
                        "Format"
                    }
                }
            }
            textarea {
                id: id,
                class: "w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring",
                style: "{min_h}",
                placeholder: placeholder.unwrap_or_default(),
                value: "{value}",
                spellcheck: false,
                oninput: move |e| on_change.call(e.value()),
            }
        }
    }
}