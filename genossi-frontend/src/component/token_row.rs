//! TokenRow Component (Phase 4 Plan 06 / W-04) — Component-First-Extraction.
//! Eine Row in der Helfer-Token-Liste mit Status-Badge + Revoke-Button + Confirm.
//!
//! ADR-2026-05-06: Open-Status-Tokens carry `code` + `qr_svg` from the
//! backend. The row renders a "QR/Code anzeigen" button that opens a Modal
//! containing the [`QrCard`] for re-display. Pre-update legacy rows have
//! `code = None` and surface an inline "revoke + recreate" hint instead.
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, HelperTokenStatusTO, HelperTokenTO};
use crate::component::{Modal, QrCard};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;

/// ADR-2026-05-06: pure helper that classifies what to render in the
/// "show code" slot for an Open-status token.
///
/// Returns:
/// - `ShowCodeMode::Available { code, qr_svg }` — show the "QR/Code anzeigen" button
/// - `ShowCodeMode::LegacyMissing` — show the "revoke + recreate" hint
///
/// Cargo-testbar — operates only on the option pair carried by the TO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowCodeMode {
    Available { code: String, qr_svg: String },
    LegacyMissing,
}

#[allow(dead_code)]
pub fn classify_show_code(token: &HelperTokenTO) -> ShowCodeMode {
    match (token.code.as_deref(), token.qr_svg.as_deref()) {
        (Some(code), Some(qr_svg)) if !code.is_empty() && !qr_svg.is_empty() => {
            ShowCodeMode::Available {
                code: code.to_string(),
                qr_svg: qr_svg.to_string(),
            }
        }
        _ => ShowCodeMode::LegacyMissing,
    }
}

#[component]
pub fn TokenRow(
    token: HelperTokenTO,
    assembly_id: Uuid,
    on_changed: EventHandler<()>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut show_revoke_confirm = use_signal(|| false);
    let mut show_qr_modal = use_signal(|| false);
    let tid = token.id;
    let memo = token.memo.clone();
    let status = token.status.clone();
    let show_mode = classify_show_code(&token);
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
    let memo_for_modal = memo.clone();
    rsx! {
        div { class: "flex items-center justify-between bg-white border border-gray-200 rounded-lg px-4 py-3",
            div { class: "flex flex-col",
                span { class: "font-medium", "{memo}" }
                if !used_at.is_empty() {
                    span { class: "text-xs text-gray-500",
                        "{i18n.t(Key::HelperTokenRedeemed)}: {used_at}"
                    }
                }
                // ADR-2026-05-06: legacy hint on Open tokens whose code is
                // missing (pre-update rows). Used/Revoked tokens skip the
                // hint since their code is no longer actionable anyway.
                if matches!(status, HelperTokenStatusTO::Open)
                    && matches!(show_mode, ShowCodeMode::LegacyMissing) {
                    span { class: "text-xs text-amber-700",
                        "{i18n.t(Key::HelperTokenCodeMissing)}"
                    }
                }
            }
            div { class: "flex items-center gap-3",
                span {
                    class: "{status_class} px-2 py-1 rounded text-xs font-medium",
                    "{status_label}"
                }
                // ADR-2026-05-06: re-display button — only for Open tokens
                // whose code is actually available.
                if matches!(status, HelperTokenStatusTO::Open)
                    && matches!(show_mode, ShowCodeMode::Available { .. }) {
                    button {
                        r#type: "button",
                        class: "text-sm text-blue-600 hover:text-blue-800 underline min-h-[44px] px-2",
                        onclick: move |_| show_qr_modal.set(true),
                        "{i18n.t(Key::HelperTokenShow)}"
                    }
                }
                if matches!(status, HelperTokenStatusTO::Open) {
                    button {
                        r#type: "button",
                        class: "text-sm text-red-600 hover:text-red-800 underline min-h-[44px] px-2",
                        onclick: move |_| show_revoke_confirm.set(true),
                        "{i18n.t(Key::HelperTokenRevoke)}"
                    }
                }
            }
        }
        if *show_qr_modal.read() {
            if let ShowCodeMode::Available { code, qr_svg } = show_mode.clone() {
                Modal {
                    div { class: "flex flex-col gap-4 items-center",
                        QrCard {
                            memo: memo_for_modal.clone(),
                            code: code,
                            qr_svg: qr_svg,
                        }
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_qr_modal.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
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
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_revoke_confirm.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            r#type: "button",
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_token(code: Option<&str>, qr: Option<&str>) -> HelperTokenTO {
        HelperTokenTO {
            id: Uuid::nil(),
            assembly_id: Uuid::nil(),
            memo: "Anna".to_string(),
            status: HelperTokenStatusTO::Open,
            used_at: None,
            revoked_at: None,
            created: None,
            version: Uuid::nil(),
            code: code.map(String::from),
            qr_svg: qr.map(String::from),
        }
    }

    #[test]
    fn classify_returns_available_for_full_payload() {
        let token = make_token(Some("ABC1234567"), Some("<svg/>"));
        let mode = classify_show_code(&token);
        match mode {
            ShowCodeMode::Available { code, qr_svg } => {
                assert_eq!(code, "ABC1234567");
                assert_eq!(qr_svg, "<svg/>");
            }
            _ => panic!("expected Available, got {:?}", mode),
        }
    }

    #[test]
    fn classify_returns_legacy_missing_when_code_is_none() {
        let token = make_token(None, None);
        assert_eq!(classify_show_code(&token), ShowCodeMode::LegacyMissing);
    }

    #[test]
    fn classify_returns_legacy_missing_when_qr_svg_is_none() {
        // Defensive: even if the backend supplies one but not the other, we
        // suppress the button — the modal needs both.
        let token = make_token(Some("ABC1234567"), None);
        assert_eq!(classify_show_code(&token), ShowCodeMode::LegacyMissing);
    }

    #[test]
    fn classify_returns_legacy_missing_when_code_is_empty() {
        let token = make_token(Some(""), Some("<svg/>"));
        assert_eq!(classify_show_code(&token), ShowCodeMode::LegacyMissing);
    }
}
