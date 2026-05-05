//! CreateTokenForm Component (Phase 4 Plan 06 / W-04) — Memo-Input + Submit.
//! Plan 08 nutzt diesen Component innerhalb eines `<Modal>`.
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, HelperTokenCreateResponseTO};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[component]
pub fn CreateTokenForm(
    assembly_id: Uuid,
    on_close: EventHandler<()>,
    on_created: EventHandler<HelperTokenCreateResponseTO>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut memo = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    // Pre-resolve i18n strings for closures (i18n is not Copy, can't be moved into multiple closures).
    let memo_required_msg = i18n.t(Key::HelperTokenMemo).to_string();
    rsx! {
        form {
            class: "flex flex-col gap-4",
            onsubmit: move |e| {
                e.prevent_default();
                let m = memo.read().trim().to_string();
                if m.is_empty() {
                    on_error.call(memo_required_msg.clone());
                    return;
                }
                submitting.set(true);
                spawn(async move {
                    let config = CONFIG.read().clone();
                    match api::create_helper_token(&config, assembly_id, m).await {
                        Ok(resp) => on_created.call(resp),
                        Err(e) => on_error.call(e.message),
                    }
                    submitting.set(false);
                });
            },
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::HelperTokenCreate)}" }
            label { class: "flex flex-col gap-1",
                span { class: "text-sm text-gray-700", "{i18n.t(Key::HelperTokenMemo)}" }
                input {
                    class: "border border-gray-300 rounded px-3 py-2",
                    r#type: "text",
                    placeholder: "{i18n.t(Key::HelperTokenMemoPlaceholder)}",
                    value: "{memo}",
                    oninput: move |e| memo.set(e.value()),
                }
            }
            p { class: "text-xs text-amber-700", "{i18n.t(Key::HelperTokenWarning)}" }
            div { class: "flex gap-2 justify-end",
                button {
                    r#type: "button",
                    class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                    onclick: move |_| on_close.call(()),
                    "{i18n.t(Key::Cancel)}"
                }
                button {
                    r#type: "submit",
                    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded disabled:opacity-50 min-h-[44px]",
                    disabled: *submitting.read(),
                    "{i18n.t(Key::HelperTokenCreate)}"
                }
            }
        }
    }
}
