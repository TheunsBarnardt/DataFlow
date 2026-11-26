use dioxus::prelude::*;

/// Modal confirmation before a destructive delete/clear action.
#[component]
pub fn ConfirmDeleteDialog(
    title: String,
    description: String,
    #[props(default = "Delete".to_string())] confirm_label: String,
    #[props(default)] busy: bool,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4 backdrop-blur-[1px]",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-sm rounded-xl border bg-card p-5 shadow-2xl",
                onclick: move |e| e.stop_propagation(),
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                h2 { class: "text-lg font-semibold", "{title}" }
                p { class: "mt-2 text-sm text-muted-foreground", "{description}" }
                div { class: "mt-5 flex justify-end gap-2",
                    button {
                        r#type: "button",
                        class: "inline-flex h-9 items-center rounded-md px-4 text-sm text-muted-foreground hover:bg-muted",
                        disabled: busy,
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-9 items-center rounded-md bg-destructive px-4 text-sm text-destructive-foreground disabled:opacity-50",
                        disabled: busy,
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_label}"
                    }
                }
            }
        }
    }
}
