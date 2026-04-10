use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

#[component]
pub fn MailBodyEditor(value: String, on_change: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 mb-1", {i18n.t(Key::MailBody)} }
            textarea {
                class: "w-full border rounded px-3 py-2 h-40",
                value: "{value}",
                oninput: move |e| on_change.call(e.value()),
            }
        }
    }
}
