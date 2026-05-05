//! TokenRow Component (Phase 4 Plan 06 / W-04) — Component-First-Extraction.
//! Eine Row in der Helfer-Token-Liste mit Status-Badge + Revoke-Button + Confirm.
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, HelperTokenStatusTO, HelperTokenTO};
use crate::component::Modal;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

#[component]
pub fn TokenRow(
    token: HelperTokenTO,
    assembly_id: Uuid,
    on_changed: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut show_revoke_confirm = use_signal(|| false);
    let tid = token.id;
    let memo = token.memo.clone();
    let status = token.status.clone();
    let (status_class, status_label) = match &status {
        HelperTokenStatusTO::Open => (
            "bg-yellow-100 text-yellow-800",
            i18n.t(Key::HelperTokenStatusOpen).to_string(),
        ),
        HelperTokenStatusTO::Used => (
            "bg-green-100 text-green-800",
            i18n.t(Key::HelperTokenStatusUsed).to_string(),
        ),
        HelperTokenStatusTO::Revoked => (
            "bg-gray-100 text-gray-500",
            i18n.t(Key::HelperTokenStatusRevoked).to_string(),
        ),
    };
    let used_at = token.used_at.clone().unwrap_or_default();
    rsx! {
        div { class: "flex items-center justify-between bg-white border border-gray-200 rounded-lg px-4 py-3",
            div { class: "flex flex-col",
                span { class: "font-medium", "{memo}" }
                if !used_at.is_empty() {
                    span { class: "text-xs text-gray-500",
                        "{i18n.t(Key::HelperTokenRedeemed)}: {used_at}"
                    }
                }
            }
            div { class: "flex items-center gap-3",
                span {
                    class: "{status_class} px-2 py-1 rounded text-xs font-medium",
                    "{status_label}"
                }
                if matches!(status, HelperTokenStatusTO::Open) {
                    button {
                        class: "text-sm text-red-600 hover:text-red-800 underline min-h-[44px] px-2",
                        onclick: move |_| show_revoke_confirm.set(true),
                        "{i18n.t(Key::HelperTokenRevoke)}"
                    }
                }
            }
        }
        if *show_revoke_confirm.read() {
            Modal {
                div { class: "flex flex-col gap-4",
                    h2 { class: "text-xl font-semibold",
                        "{i18n.t(Key::HelperTokenRevokeConfirmTitle)}"
                    }
                    p { class: "text-sm text-gray-700",
                        "{i18n.t(Key::HelperTokenRevokeConfirmText)}"
                    }
                    div { class: "flex gap-2 justify-end mt-2",
                        button {
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_revoke_confirm.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            class: "bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| {
                                show_revoke_confirm.set(false);
                                spawn(async move {
                                    let config = CONFIG.read().clone();
                                    match api::revoke_helper_token(&config, assembly_id, tid).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(e) => on_error.call(e.message),
                                    }
                                });
                            },
                            "{i18n.t(Key::HelperTokenRevoke)}"
                        }
                    }
                }
            }
        }
    }
}
