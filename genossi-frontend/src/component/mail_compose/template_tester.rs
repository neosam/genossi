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
//! Phase 28 (PREV-02, D-03): Die Mitglieds-Auswahl wird seit Phase 28 NICHT
//! mehr hier lokal gehalten, sondern von der Page als Prop `selected_member_id`
//! hereingereicht — dieselbe Auswahl speist zugleich die Device-Vorschau im
//! `WysiwygEditor` und die `TemplatePreview`. Damit verschwindet die frühere
//! doppelte Mitglieds-Auswahl im Template-Editor. An den drei
//! Datenschutz-Schichten oben ändert das NICHTS: das jetzt von außen
//! gesteuerte Signal liefert weiterhin ausschließlich die Template-Variablen
//! über seine Id und gerät an keiner Stelle in den Empfänger-Pfad (T-28-19).
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
    // Phase 28 (PREV-02, D-03): Member-Auswahl kommt von der Page, damit sie
    // zugleich die Device-Vorschau im `WysiwygEditor` speist. Bewusst KEIN
    // `#[props(default)]` (T-28-23): ein Default würde bei jedem Render der
    // Elternkomponente ein neues Signal im Eltern-Scope anlegen, Zustand
    // verlieren und Signale akkumulieren.
    //
    // PRIVACY: dieses Prop liefert ausschließlich die Template-Variablen über
    // seine Id. Es darf NIE in den Empfänger-Pfad der Test-Mail geraten — der
    // Empfänger stammt allein aus `test_address` (siehe Modul-Doc, Schicht 2).
    mut selected_member_id: Signal<Option<Uuid>>,
) -> Element {
    let i18n = use_i18n();
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
                    // Phase 28 (PREV-02, D-03): dasselbe Signal wie die
                    // MemberSearch oben — die Auswahl im Auswahlfeld der
                    // Vorschau und die Suche hier zeigen ab jetzt denselben
                    // Zustand. Genau das beendet die doppelte Auswahl.
                    preview_member_id: selected_member_id,
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

// Grep-gate tests below — muss die LETZTE Modul-Deklaration der Datei bleiben,
// weil `production_region()` alles ab dem Marker abschneidet. Self-Reference-
// Abwehr zweischichtig wie in `wysiwyg_editor.rs` und `template_preview.rs`:
// (a) Source vor dem Marker abschneiden, (b) Needles zur Laufzeit
// zusammensetzen. Kein Modul-Doc mit Ziel-Literalen, damit keine der Needles
// versehentlich in der Produktionsregion landet.
#[cfg(test)]
mod grep_gate_tests {
    const TESTER_SRC: &str = include_str!("template_tester.rs");
    const TEST_MODULE_MARKER: &str = "mod grep_gate_tests";

    fn production_region() -> &'static str {
        let cutoff = TESTER_SRC
            .find(TEST_MODULE_MARKER)
            .expect("BUG: grep-gate test module marker not found");
        &TESTER_SRC[..cutoff]
    }

    /// T-28-19 — DATENSCHUTZ. Die Test-Mail darf NIEMALS an die E-Mail-Adresse
    /// des gewählten Mitglieds gehen, sondern ausschließlich an die im
    /// Test-Empfänger-Feld eingetragene Adresse. Seit Phase 28 wird die
    /// Mitglieds-Auswahl von der Page hereingereicht; dieser Gate nagelt fest,
    /// dass sie dadurch nicht in den Empfänger-Pfad rutscht.
    ///
    /// Fenster-Suche: das erste Argument nach der Sende-Funktion muss die aus
    /// dem Test-Adress-Feld gelesene Variable sein.
    #[test]
    fn test_mail_recipient_comes_from_test_address_only() {
        let region = production_region();
        let send_needle = format!("send_test_mail_with_templat{tail}", tail = "e(");
        let addr_needle = format!("&add{tail}", tail = "r,");
        let idx = region.find(&send_needle).expect(
            "Grep gate FAILED: der Aufruf der Test-Mail-Sendefunktion fehlt in \
             template_tester.rs (Produktionsregion) komplett.",
        );
        let window = &region[idx..idx.saturating_add(200).min(region.len())];
        assert!(
            window.contains(&addr_needle),
            "Grep gate FAILED: die Empfängeradresse der Test-Mail stammt nicht \
             mehr aus dem Test-Adress-Feld. DATENSCHUTZREGEL (Genossi, \
             Datensparsamkeit): die Test-Mail darf niemals an die Adresse des \
             gewählten Mitglieds gehen — das Mitglied liefert ausschließlich die \
             Template-Variablen über seine Id. Fenster hinter dem Sende-Aufruf \
             (erste 200 Zeichen):\n{window}"
        );
    }

    /// Phase 28 (PREV-02, D-03) — `TemplateTester` muss sein von der Page
    /// hereingereichtes Signal tatsächlich an die `TemplatePreview` weitergeben.
    /// Tut es das nicht, hätte der Template-Editor wieder zwei unabhängige
    /// Mitglieds-Auswahlen auf derselben Seite — exakt das Problem, das D-03
    /// behebt.
    #[test]
    fn selected_member_id_is_forwarded_to_preview() {
        let region = production_region();
        let forward_needle = format!("preview_member_i{tail}", tail = "d:");
        assert!(
            region.contains(&forward_needle),
            "Grep gate FAILED: template_tester.rs reicht seine Member-Auswahl \
             nicht mehr an TemplatePreview weiter. Damit stünden im \
             Template-Editor wieder zwei konkurrierende Mitglieds-Auswahlen \
             nebeneinander (Phase 28, D-03)."
        );
    }

    #[test]
    fn production_region_excludes_test_module() {
        let region = production_region();
        assert!(
            !region.contains(TEST_MODULE_MARKER),
            "BUG: production_region() slice still contains the test module marker"
        );
        assert!(region.len() < TESTER_SRC.len());
    }
}
