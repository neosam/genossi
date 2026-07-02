//! Quick 260603-jtf — TemplateTester component.
//!
//! Reusable "Test template" widget for the Mail-Template editor (and
//! potentially the Mail-Compose page later). Composes:
//!   - `MemberSearch` to pick exactly one Member whose data drives the
//!     rendered preview/test-mail variables
//!   - `TemplatePreview` (reused 1:1, never duplicated) to show the
//!     rendered Subject+Body for that Member
//!   - An **explicit** Test-Adress-Input and "Send test mail" button so the
//!     test mail is dispatched to the test recipient — NEVER to the Member's
//!     own email address (datasparsame Genossi privacy rule).
//!
//! Privacy defense layers (all three must hold):
//!  1. UI: the Test-Empfänger input is a separate `<input type="email">`,
//!     visually distinct from the Member selector, with an amber hint that
//!     explains the recipient is NOT the Member.
//!  2. Frontend wiring: `onclick` reads ONLY `test_address` for `to_address`
//!     (see doc-comment above the `onclick` handler below).
//!  3. Backend: `POST /api/mail/test-with-template` requires both fields and
//!     forwards `body.to_address` (NEVER `member.email`) to
//!     `MailService::send_test_mail_with_body`.
//!
//! Tests at the bottom verify the pure helper `is_valid_test_address`.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api;
use crate::component::mail_compose::TemplatePreview;
use crate::component::member_search::MemberSearch;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::service::member::refresh_members;

/// Bewusst minimale Address-Validation — keine RFC5321-Vollparser-Validation.
/// Lettre rejected ungültige Adressen serverseitig mit `502 SmtpError`; das
/// reicht für die UX (Button stays disabled while empty, server errors get
/// surfaced in the feedback toast).
pub(crate) fn is_valid_test_address(addr: &str) -> bool {
    let trimmed = addr.trim();
    !trimmed.is_empty() && trimmed.contains('@')
}

#[component]
pub fn TemplateTester(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    // Phase 24 Plan 03 Task 4 (EDIT-01, D-01): HTML sibling of `body` —
    // forwarded to TemplatePreview so the Live-Preview renders the
    // backend's HTML sibling (Phase 24 Plan 01 Task 1 extended preview_mail).
    // Defaults to an empty ReadOnlySignal via #[props(default)] so existing
    // callers stay source-compat.
    #[props(default)] body_html: ReadOnlySignal<String>,
) -> Element {
    let i18n = use_i18n();
    let mut selected_member_id = use_signal(|| None::<Uuid>);
    let mut test_address = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<(bool, String)>);

    // Mount-Hook: ensure MEMBERS signal is populated so MemberSearch and
    // TemplatePreview have data to render. Same pattern as mail_page.rs.
    use_effect(move || {
        spawn(async move {
            refresh_members().await;
        });
    });

    let send_disabled = *sending.read()
        || !is_valid_test_address(&test_address.read())
        || selected_member_id.read().is_none();

    rsx! {
        div { class: "bg-gray-50 rounded-lg p-4 mt-4",
            h3 { class: "text-sm font-medium text-gray-700 mb-2",
                {i18n.t(Key::MailTemplateTest)}
            }

            // Member selector (drives template variables only — never the recipient).
            div { class: "mb-3",
                MemberSearch {
                    on_select: move |id: Option<Uuid>| {
                        selected_member_id.set(id);
                        // Reset feedback when changing member.
                        feedback.set(None);
                    },
                    selected_id: *selected_member_id.read(),
                    exclude_id: None,
                }
            }

            // Live preview re-uses the existing TemplatePreview component
            // (Component-First: NEVER duplicate the render-trigger or
            // dropdown logic here).
            if let Some(mid) = *selected_member_id.read() {
                TemplatePreview {
                    subject: subject,
                    body: body,
                    body_html: body_html,
                    member_ids: vec![mid],
                }
            }

            // Test-recipient input + send button (explicit, separate from
            // the member selector).
            div { class: "mt-3 border-t pt-3",
                label { class: "block text-xs font-medium text-gray-500 mb-1",
                    {i18n.t(Key::MailTemplateTestSendTo)}
                }
                p { class: "text-xs text-amber-600 mb-2",
                    {i18n.t(Key::MailTemplateTestPrivacyHint)}
                }
                input {
                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 text-sm mb-2",
                    r#type: "email",
                    placeholder: "test@example.com",
                    value: "{test_address}",
                    oninput: move |e| test_address.set(e.value()),
                }
                button {
                    // Memory `feedback_dioxus_button_type.md`: ohne
                    // r#type="button" reloadet die Page trotz prevent_default.
                    r#type: "button",
                    class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: send_disabled,
                    // PRIVACY: to_address kommt AUSSCHLIESSLICH aus dem
                    // test_address-Signal, NIE aus member.email. Member liefert
                    // nur die Template-Variablen via member_id im Request —
                    // Backend (genossi_mail/src/rest.rs::send_test_mail_with_template)
                    // rendert mit member-Context und sendet an body.to_address.
                    onclick: move |_| {
                        let Some(mid) = *selected_member_id.read() else { return; };
                        let addr = test_address.read().trim().to_string();
                        if !is_valid_test_address(&addr) {
                            return;
                        }
                        let subj = subject.read().clone();
                        let bdy = body.read().clone();
                        let mid_str = mid.to_string();
                        spawn(async move {
                            sending.set(true);
                            feedback.set(None);
                            let config = CONFIG.read().clone();
                            match api::send_test_mail_with_template(
                                &config, &addr, &subj, &bdy, &mid_str,
                            )
                            .await
                            {
                                Ok(()) => feedback.set(Some((false, String::new()))),
                                Err(e) => feedback.set(Some((true, e.to_string()))),
                            }
                            sending.set(false);
                        });
                    },
                    if *sending.read() {
                        {i18n.t(Key::MailSending)}
                    } else {
                        {i18n.t(Key::MailTemplateTestSend)}
                    }
                }
            }

            // Feedback toast.
            if let Some((is_error, msg)) = feedback.read().clone() {
                if is_error {
                    div { class: "mt-3 bg-red-50 border border-red-200 rounded p-3 text-sm text-red-700",
                        p { class: "font-medium", {i18n.t(Key::MailTemplateTestFailed)} }
                        p { "{msg}" }
                    }
                } else {
                    div { class: "mt-3 bg-green-50 border border-green-200 rounded p-3 text-sm text-green-700",
                        {i18n.t(Key::MailTemplateTestSuccess)}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quick 260603-jtf: a normal `"user@host.tld"`-shaped string is accepted.
    #[test]
    fn test_is_valid_test_address_accepts_normal() {
        assert!(is_valid_test_address("vorstand@example.com"));
    }

    /// Quick 260603-jtf: empty strings, whitespace-only strings, and strings
    /// without an `@` sign are rejected. This keeps the send-button disabled
    /// until the user supplies an actual email address; the backend then
    /// performs the real validation via lettre.
    #[test]
    fn test_is_valid_test_address_rejects_empty_and_missing_at() {
        assert!(!is_valid_test_address(""));
        assert!(!is_valid_test_address("   "));
        assert!(!is_valid_test_address("no-at-sign"));
    }

    /// Quick 260603-jtf: leading/trailing whitespace is trimmed before the
    /// `@`-check so a user pasting an address with extra spaces still gets
    /// the send-button enabled.
    #[test]
    fn test_is_valid_test_address_trims() {
        assert!(is_valid_test_address("  foo@bar.de  "));
    }
}
