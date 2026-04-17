use dioxus::prelude::*;

use crate::api::AppError;

#[component]
pub fn ErrorAlert(error: AppError, on_dismiss: Option<EventHandler<()>>) -> Element {
    let mut show_details = use_signal(|| false);

    rsx! {
        div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4 relative",
            div { class: "flex items-start justify-between",
                div { class: "flex-1",
                    p { class: "font-medium", "{error.message}" }
                }
                if let Some(handler) = on_dismiss {
                    button {
                        r#type: "button",
                        class: "ml-4 text-red-500 hover:text-red-700 font-bold text-lg leading-none",
                        onclick: move |_| handler.call(()),
                        "\u{00D7}"
                    }
                }
            }
            if let Some(ref detail) = error.detail {
                div { class: "mt-2",
                    button {
                        r#type: "button",
                        class: "text-sm text-red-600 underline hover:text-red-800",
                        onclick: move |_| {
                            let current = *show_details.read();
                            show_details.set(!current);
                        },
                        if *show_details.read() {
                            "Details ausblenden"
                        } else {
                            "Details anzeigen"
                        }
                    }
                    if *show_details.read() {
                        pre { class: "mt-2 text-xs bg-red-50 p-2 rounded overflow-x-auto whitespace-pre-wrap break-words",
                            "{detail}"
                        }
                    }
                }
            }
        }
    }
}
