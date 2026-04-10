use dioxus::prelude::*;

use super::InboxStatusBadge;

#[component]
pub fn InboxMailListItem(
    subject: String,
    from_address: String,
    received_at: String,
    status: String,
    has_attachments: bool,
    assigned_label: String,
    selected: bool,
    on_click: EventHandler<()>,
) -> Element {
    let row_class = if selected {
        "p-3 cursor-pointer bg-blue-50"
    } else {
        "p-3 cursor-pointer hover:bg-gray-50"
    };

    rsx! {
        li {
            class: "{row_class}",
            onclick: move |_| on_click.call(()),
            div { class: "flex justify-between",
                span { class: "font-medium truncate", "{subject}" }
                InboxStatusBadge { status: status }
            }
            div { class: "text-sm text-gray-600 truncate", "{from_address}" }
            div { class: "flex justify-between text-xs text-gray-500",
                span { "{received_at}" }
                span {
                    if has_attachments { "📎 " } else { "" }
                    "{assigned_label}"
                }
            }
        }
    }
}
