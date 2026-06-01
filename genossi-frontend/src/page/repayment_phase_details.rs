//! Repayment phase detail page (Phase 12 Plan 12-05, UI-02) — admin-only.
//!
//! Layout: TabStrip mit 3 fixen Tabs (Stammdaten / Einträge / Export — D-06).
//! Header zeigt Titel + Status-Badge OHNE Lifecycle-Buttons (D-03).
//! Stamm-Tab hat die Lifecycle-Action-Tile (D-03) mit Öffnen + Schließen.
//! Schließen hat Confirm-Modal (D-07). Öffnen NICHT (reversibel über Edit/Delete der Entries).
//!
//! Status-driven render:
//!   Preparation → Einträge-Tab + Export-Tab zeigen "Phase noch nicht geöffnet"
//!   Open        → alle Tabs aktiv; Schließen-Button sichtbar
//!   Closed      → alle Felder read-only (D-08); kein Lifecycle-Button
//!
//! Plan 12-06 erweitert um `share_value`-Inline-Edit (D-05).
//! Plan 12-08 ersetzt EntriesTab-Stub mit RepaymentEntryList.
//! Plan 12-14 ersetzt ExportTab-Stub mit Include-Filter + PDF-Download.

use dioxus::prelude::*;
use std::str::FromStr;
use uuid::Uuid;

use crate::api::{
    self, AppError, CloseConflictResponse, RepaymentPhaseStatusTO, RepaymentPhaseTO,
};
use crate::auth::RequirePrivilege;
use crate::component::repayment_format::format_payout_eur;
use crate::component::{
    ErrorAlert, Modal, RepaymentPhaseStatusBadge, TabDef, TabStrip, ToastContainer, TopBar,
    show_toast,
};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::service::config::CONFIG;

/// D-03 + D-08: Öffnen-Button is only visible in status `Preparation`.
fn should_show_open_button(status: RepaymentPhaseStatusTO) -> bool {
    matches!(status, RepaymentPhaseStatusTO::Preparation)
}

/// D-03 + D-08: Schließen-Button is only visible in status `Open`.
fn should_show_close_button(status: RepaymentPhaseStatusTO) -> bool {
    matches!(status, RepaymentPhaseStatusTO::Open)
}

/// D-05 + D-08: `share_value` is read-only in status `Closed`.
/// Plan 12-06 reuses this for the inline-edit guard.
pub(crate) fn is_share_value_editable(status: RepaymentPhaseStatusTO) -> bool {
    !matches!(status, RepaymentPhaseStatusTO::Closed)
}

/// D-04 + Open-Question 5: parses the 409-detail body of POST /api/repayment-phase/{id}/close
/// into a CloseConflictResponse. Returns None when the body is not a valid
/// CloseConflictResponse (e.g. non-409 errors, missing detail body, or garbled JSON).
/// On Some, the caller renders an ErrorAlert with pending_count + member-number list
/// (detail-expand). On None, the caller falls back to a generic Toast with the error message.
fn parse_close_conflict(err: &AppError) -> Option<CloseConflictResponse> {
    if err.status != Some(409) {
        return None;
    }
    let body = err.detail.as_deref()?;
    serde_json::from_str::<CloseConflictResponse>(body).ok()
}

#[component]
pub fn RepaymentPhaseDetails(id: String) -> Element {
    let i18n = use_i18n();
    let phase_id = match Uuid::from_str(&id) {
        Ok(u) => u,
        Err(_) => return rsx! { div { class: "p-4 text-red-600", "Invalid phase id" } },
    };

    let mut phase = use_signal(|| Option::<RepaymentPhaseTO>::None);
    let mut loading = use_signal(|| true);
    let mut active_tab = use_signal(|| "basics".to_string());
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);
    // D-04 + Open-Question 5: 409 CloseConflictResponse via ErrorAlert mit Detail-Expand
    let mut close_conflict = use_signal(|| Option::<CloseConflictResponse>::None);

    let load_phase = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_repayment_phase(&config, phase_id).await {
                Ok(p) => phase.set(Some(p)),
                Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_phase();
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            TopBar {}
            div { class: "container mx-auto px-4 py-6",
                if *loading.read() {
                    p { class: "text-gray-500 text-center py-8", "{i18n.t(Key::Loading)}" }
                } else if let Some(p) = phase.read().clone() {
                    // D-03: Header mit Titel + Status-Badge, KEINE Lifecycle-Buttons
                    div { class: "flex items-center justify-between mb-4",
                        h1 { class: "text-2xl font-bold",
                            "{i18n.t(Key::RepaymentPhases)} {p.fiscal_year}"
                        }
                        RepaymentPhaseStatusBadge { status: p.status }
                    }

                    // D-04: 409 CloseConflictResponse als ErrorAlert mit Detail-Expand
                    if let Some(cc) = close_conflict.read().clone() {
                        ErrorAlert {
                            error: AppError {
                                status: Some(409),
                                message: format!(
                                    "{}: {} Einträge noch nicht ausbezahlt",
                                    i18n.t(Key::RepaymentPhaseCloseBlocked),
                                    cc.pending_count,
                                ),
                                detail: Some(format!(
                                    "Betroffene Mitglieder: {}",
                                    cc.pending_member_numbers.join(", ")
                                )),
                            },
                            on_dismiss: Some(EventHandler::new(move |_| close_conflict.set(None))),
                        }
                    }

                    // D-06: TabStrip mit IMMER 3 sichtbaren Tabs (anders als assembly_details.rs)
                    {
                        let tab_defs = vec![
                            TabDef { key: "basics",  label: i18n.t(Key::RepaymentPhaseTabBasics).to_string() },
                            TabDef { key: "entries", label: i18n.t(Key::RepaymentPhaseTabEntries).to_string() },
                            TabDef { key: "export",  label: i18n.t(Key::RepaymentPhaseTabExport).to_string() },
                        ];
                        let active_key = active_tab.read().clone();
                        let status_value = p.status;
                        let phase_for_basics = p.clone();
                        let phase_for_entries = p.clone();
                        let phase_for_export = p.clone();
                        rsx! {
                            TabStrip {
                                tabs: tab_defs,
                                active_key: active_key.clone(),
                                on_change: move |k: String| active_tab.set(k),
                                match active_key.as_str() {
                                    "basics" => rsx! {
                                        BasicsTab {
                                            phase: phase_for_basics,
                                            on_changed: move |_| load_phase(),
                                            on_close_conflict: move |cc: CloseConflictResponse| close_conflict.set(Some(cc)),
                                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                        }
                                    },
                                    "entries" => match status_value {
                                        RepaymentPhaseStatusTO::Preparation => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "{i18n.t(Key::RepaymentEntriesNotOpenYet)}"
                                            }
                                        },
                                        // Plan 12-08 ersetzt diesen Stub mit RepaymentEntryList
                                        _ => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "TODO Plan 12-08: RepaymentEntryList für phase_id={phase_for_entries.id}"
                                            }
                                        },
                                    },
                                    "export" => match status_value {
                                        RepaymentPhaseStatusTO::Preparation => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "{i18n.t(Key::RepaymentExportNotOpenYet)}"
                                            }
                                        },
                                        // Plan 12-14 ersetzt diesen Stub mit Format-Picker + Download
                                        _ => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "TODO Plan 12-14: Export-Tab für phase_id={phase_for_export.id}"
                                            }
                                        },
                                    },
                                    _ => rsx! {},
                                }
                            }
                        }
                    }
                } else {
                    p { class: "text-red-600 text-center py-8", "Phase not found" }
                }
            }
            ToastContainer { messages: toast_messages }
        }
    }
}

/// Stamm-Daten-Tab — Read-Only-Display in Wave 3.
/// Plan 12-06 erweitert um `share_value`-Inline-Edit (D-05).
#[component]
fn BasicsTab(
    phase: RepaymentPhaseTO,
    on_changed: EventHandler<()>,
    on_close_conflict: EventHandler<CloseConflictResponse>,
    on_error: EventHandler<String>,
) -> Element {
    let i18n = use_i18n();
    let mut show_close_confirm = use_signal(|| false);
    let phase_id = phase.id;
    let phase_status = phase.status;

    rsx! {
        div { class: "flex flex-col gap-6",
            // Stamm-Daten-Block (read-only in Wave 3; Plan 12-06 erweitert)
            div { class: "grid grid-cols-2 gap-4",
                div {
                    span { class: "text-sm text-gray-500", "{i18n.t(Key::RepaymentPhaseFiscalYear)}" }
                    p { class: "text-lg font-semibold", "{phase.fiscal_year}" }
                }
                div {
                    span { class: "text-sm text-gray-500", "{i18n.t(Key::RepaymentPhaseShareValue)}" }
                    p { class: "text-lg font-semibold", "{format_payout_eur(1, phase.share_value)}" }
                }
            }

            // D-03 + D-08: Lifecycle-Action-Tile als große Kachel
            if should_show_open_button(phase_status) {
                div { class: "rounded border border-gray-200 bg-blue-50 p-4 flex items-center justify-between",
                    div {
                        p { class: "font-semibold text-blue-900", "{i18n.t(Key::RepaymentPhaseOpen)}" }
                        p { class: "text-sm text-blue-700",
                            "Beim Öffnen werden alle Vorjahres-Austritte als Einträge angelegt."
                        }
                    }
                    button {
                        r#type: "button",
                        class: "bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded min-h-[44px]",
                        onclick: move |_| {
                            spawn(async move {
                                let config = CONFIG.read().clone();
                                match api::open_repayment_phase(&config, phase_id).await {
                                    // Pitfall #7 / Phase 8 CR-01: re-fetch via on_changed
                                    // statt Response-Body-Version zu verwenden.
                                    // D-09: KEIN Auto-Tab-Switch — on_changed lädt nur die Phase neu
                                    Ok(_) => on_changed.call(()),
                                    Err(err) => on_error.call(err.message),
                                }
                            });
                        },
                        "{i18n.t(Key::RepaymentPhaseOpen)}"
                    }
                }
            }

            if should_show_close_button(phase_status) {
                div { class: "rounded border border-gray-200 bg-red-50 p-4 flex items-center justify-between",
                    div {
                        p { class: "font-semibold text-red-900", "{i18n.t(Key::RepaymentPhaseClose)}" }
                        p { class: "text-sm text-red-700",
                            "Schließen geht nur, wenn alle Einträge ausbezahlt oder gelöscht sind."
                        }
                    }
                    button {
                        r#type: "button",
                        class: "bg-red-600 hover:bg-red-700 text-white px-6 py-3 rounded min-h-[44px]",
                        onclick: move |_| show_close_confirm.set(true),
                        "{i18n.t(Key::RepaymentPhaseClose)}"
                    }
                }
            }
        }

        // D-07: Confirm-Modal vor Close-POST
        if *show_close_confirm.read() {
            Modal {
                div { class: "flex flex-col gap-4",
                    h2 { class: "text-xl font-semibold", "{i18n.t(Key::RepaymentPhaseCloseConfirmTitle)}" }
                    p { class: "text-sm text-gray-700", "{i18n.t(Key::RepaymentPhaseCloseConfirmText)}" }
                    div { class: "flex gap-2 justify-end mt-2",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                            onclick: move |_| show_close_confirm.set(false),
                            "{i18n.t(Key::Cancel)}"
                        }
                        button {
                            r#type: "button",
                            class: "bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded min-h-[44px]",
                            onclick: move |_| {
                                show_close_confirm.set(false);
                                spawn(async move {
                                    let config = CONFIG.read().clone();
                                    match api::close_repayment_phase(&config, phase_id).await {
                                        Ok(_) => on_changed.call(()),
                                        Err(err) => {
                                            // D-04: 409 mit CloseConflictResponse strukturiert anzeigen
                                            if let Some(cc) = parse_close_conflict(&err) {
                                                on_close_conflict.call(cc);
                                            } else {
                                                on_error.call(err.message);
                                            }
                                        }
                                    }
                                });
                            },
                            "{i18n.t(Key::RepaymentPhaseClose)}"
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

    #[test]
    fn open_button_only_in_preparation() {
        assert!(should_show_open_button(RepaymentPhaseStatusTO::Preparation));
        assert!(!should_show_open_button(RepaymentPhaseStatusTO::Open));
        assert!(!should_show_open_button(RepaymentPhaseStatusTO::Closed));
    }

    #[test]
    fn close_button_only_in_open() {
        assert!(!should_show_close_button(RepaymentPhaseStatusTO::Preparation));
        assert!(should_show_close_button(RepaymentPhaseStatusTO::Open));
        assert!(!should_show_close_button(RepaymentPhaseStatusTO::Closed));
    }

    #[test]
    fn share_value_readonly_in_closed() {
        assert!(is_share_value_editable(RepaymentPhaseStatusTO::Preparation));
        assert!(is_share_value_editable(RepaymentPhaseStatusTO::Open));
        assert!(!is_share_value_editable(RepaymentPhaseStatusTO::Closed));
    }

    #[test]
    fn parse_close_conflict_returns_none_on_non_409() {
        let err = AppError {
            status: Some(404),
            message: "Not found".into(),
            detail: None,
        };
        assert!(parse_close_conflict(&err).is_none());
    }

    #[test]
    fn parse_close_conflict_returns_none_when_detail_missing() {
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: None,
        };
        assert!(parse_close_conflict(&err).is_none());
    }

    #[test]
    fn parse_close_conflict_returns_some_on_valid_body() {
        let body = r#"{"error":"pending entries","pending_count":2,"pending_member_numbers":["42","43"]}"#;
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: Some(body.to_string()),
        };
        let cc = parse_close_conflict(&err).expect("should parse");
        assert_eq!(cc.pending_count, 2);
        assert_eq!(
            cc.pending_member_numbers,
            vec!["42".to_string(), "43".to_string()]
        );
    }

    #[test]
    fn parse_close_conflict_returns_none_on_garbled_body() {
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: Some("not json".into()),
        };
        assert!(parse_close_conflict(&err).is_none());
    }
}
