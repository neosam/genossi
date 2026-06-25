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
    self, AppError, CloseConflictResponse, RepaymentEntryTO, RepaymentPhaseStatusTO,
    RepaymentPhaseTO,
};
use crate::auth::RequirePrivilege;
use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents};
use crate::component::{
    show_toast, ErrorAlert, Modal, RepaymentEntryAddModal, RepaymentEntryList,
    RepaymentEntryPaidOutConfirm, RepaymentLetterDownloadButton, RepaymentPhaseStatusBadge, TabDef,
    TabStrip, ToastContainer, TopBar,
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

/// Phase 12 D-18 (Plan 12-13): Baut die Redirect-URL fuer Mail-Page aus Repayment-Kontext.
///
/// UUIDs sind URL-safe (nur 0-9a-f und Bindestriche) — kein URL-Encoding noetig.
/// Bei leerer member_ids-Liste wird der members-Param weggelassen (defensive — Button
/// sollte bei 0 Selection disabled sein; siehe Plan 12-08 RepaymentEntryList Header).
///
/// Format: `/mail?from=repayment&phase_id={uuid}&members={uuid,uuid,...}`
pub(crate) fn build_mail_redirect_url(phase_id: Uuid, member_ids: &[Uuid]) -> String {
    let members_csv: String = member_ids
        .iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",");
    if members_csv.is_empty() {
        format!("/mail?from=repayment&phase_id={phase_id}")
    } else {
        format!("/mail?from=repayment&phase_id={phase_id}&members={members_csv}")
    }
}

/// Plan 12-14 / Phase 11 D-03: Filter-Optionen für den PDF-Export.
///
/// `Open` ist Backend-Default (entspricht "?include=open" — offene +
/// angeschriebene Einträge; Banking-Vorlage). `All` zieht zusätzlich
/// ausbezahlte Einträge mit ein, `Paid` filtert auf ausbezahlte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportInclude {
    Open,
    All,
    Paid,
}

impl ExportInclude {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ExportInclude::Open => "open",
            ExportInclude::All => "all",
            ExportInclude::Paid => "paid",
        }
    }
}

/// Plan 12-14 (Backend Phase 11): Vollständige URL für den PDF-Export.
///
/// Browser-native Download via Content-Disposition (Backend setzt
/// `attachment; filename="auszahlung-{fiscal_year}-{include}.pdf"`).
/// Defensives Trim eines möglichen Trailing-Slash in `backend`, damit
/// keine doppelten Schrägstriche entstehen, falls die Config einen
/// Slash am Ende behält.
pub(crate) fn build_export_url(phase_id: Uuid, include: ExportInclude, backend: &str) -> String {
    let backend_trimmed = backend.trim_end_matches('/');
    format!(
        "{backend_trimmed}/api/repayment-phase/{phase_id}/export/pdf?include={}",
        include.as_str()
    )
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
    // Plan 12-09: Add-Entry-Modal-Mount + Reload-Trigger fuer RepaymentEntryList
    let mut show_add_modal = use_signal(|| false);
    // Plan 12-09 — Counter-Trigger fuer RepaymentEntryList Re-Fetch nach
    // externer Mutation (z.B. Add-Modal-on_created). Counter-Pattern statt
    // load_phase-Cascade-Hoffen — deterministisch, kein Race mit Phase-Re-Mount.
    let mut entries_reload_trigger = use_signal(|| 0_u64);
    // Plan 12-10: PaidOut-Bulk-Confirm-Modal — Some(Vec<entries>) = sichtbar,
    // None = hidden. RepaymentEntryList ruft on_paidout_request mit der vollen
    // Liste der ausgewaehlten Eintraege; die Detail-Page entscheidet ueber Modal-Open.
    let mut paidout_modal_entries = use_signal(|| Option::<Vec<RepaymentEntryTO>>::None);

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
            privilege: crate::auth::PRIVILEGE_ADMIN,
            fallback: rsx! { AccessDeniedPage { required_privilege: crate::auth::PRIVILEGE_ADMIN.to_string() } },
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

                    // Quick 260602-sgp: Bulk-Download bereits persistierter
                    // RepaymentLetter-PDFs. Nur rendern wenn Phase NICHT in
                    // Preparation ist (Backend-Status-Gate ist hier gespiegelt
                    // damit der User nicht aufs 409 lauft).
                    if !matches!(p.status, RepaymentPhaseStatusTO::Preparation) {
                        div { class: "mb-4 p-3 bg-gray-50 rounded border",
                            RepaymentLetterDownloadButton {
                                phase_id,
                                fiscal_year: p.fiscal_year,
                                on_toast: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                            }
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
                        // Phase 13: fiscal_year fuer Browser-Save-Filename
                        // des Bundle-PDFs (`auszahlungs_anschreiben_GJ_{year}.pdf`).
                        let fiscal_year_for_letters = p.fiscal_year;
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
                                        // Plan 12-08: RepaymentEntryList wired-up.
                                        // Plan 12-09: on_add verdrahtet Add-Modal + reload_trigger Counter.
                                        // Plans 12-10/12-13 ersetzen die verbliebenen 2 Toast-Placeholder.
                                        _ => rsx! {
                                            RepaymentEntryList {
                                                phase: phase_for_entries,
                                                reload_trigger: *entries_reload_trigger.read(),
                                                on_changed: move |_| load_phase(),
                                                on_add: move |_| show_add_modal.set(true),
                                                on_paidout_request: move |entries: Vec<RepaymentEntryTO>| {
                                                    // Plan 12-10: Modal-Open mit kompletter Auswahl-Liste.
                                                    paidout_modal_entries.set(Some(entries));
                                                },
                                                on_mail_request: move |ids: Vec<uuid::Uuid>| {
                                                    // Plan 12-13 D-18: Mail-Redirect mit Repayment-Kontext.
                                                    // Build /mail?from=repayment&phase_id=...&members=...
                                                    // und navigiere via Browser Full-Page-Reload statt SPA-Push:
                                                    // dioxus-router Route::MailPage {} kennt keine Query-Param-Felder;
                                                    // mail_page.rs parst Query-Params in use_effect (Plan 12-12),
                                                    // das geht nach Full-Reload sauber.
                                                    if ids.is_empty() {
                                                        return; // defensive — Button sollte bei 0 Selection disabled sein
                                                    }
                                                    let url = build_mail_redirect_url(phase_id, &ids);
                                                    if let Some(window) = web_sys::window() {
                                                        let _ = window.location().set_href(&url);
                                                    }
                                                },
                                                on_letter_request: move |entry_ids: Vec<uuid::Uuid>| {
                                                    // Phase 13 D-13-02: Direct-Download des Bundle-PDFs +
                                                    // Browser-Save via <a download> + Toast mit Server-Document-Count.
                                                    //
                                                    // KEIN entry_ids.len() vorab speichern — wir nutzen den
                                                    // Server-aggregierten document_count aus X-Document-Count (D-13-04),
                                                    // damit "1 Brief erzeugt" auch dann stimmt, wenn der Vorstand
                                                    // 3 Entries fuer 1 Member ausgewaehlt hat.
                                                    if entry_ids.is_empty() {
                                                        return; // defensive — Button ist disabled bei 0 Selection
                                                    }
                                                    // WR-02: Client-Side Bulk-Limit-Guard (mirrors backend
                                                    // MAX_ENTRY_IDS_PER_REQUEST=200 in
                                                    // genossi_service_impl/src/repayment_letter.rs). Statt
                                                    // einer generischen 400-Toast-Message bekommt der Vorstand
                                                    // eine klare i18n-Fehlermeldung, die die Maximalanzahl nennt.
                                                    const MAX_LETTER_BULK: usize = 200;
                                                    if entry_ids.len() > MAX_LETTER_BULK {
                                                        let msg = i18n
                                                            .t(Key::RepaymentLetterBulkLimitExceeded)
                                                            .replace("{max}", &MAX_LETTER_BULK.to_string());
                                                        show_toast(&mut toast_messages, &mut toast_counter, msg);
                                                        return;
                                                    }
                                                    let phase_id_for_spawn = phase_id;
                                                    let fiscal_year_for_spawn = fiscal_year_for_letters;
                                                    // CR-01 fix: i18n-Strings AM TOP-LEVEL des Components-Renders
                                                    // resolven (use_i18n() ist ein Hook und darf NICHT in einer
                                                    // async spawn-Closure laufen). Strings via move-capture in
                                                    // die Closure — Pattern wie in BasicsTab/ExportTab.
                                                    let toast_singular = i18n.t(Key::RepaymentLetterToastSingular).to_string();
                                                    let toast_plural_template = i18n.t(Key::RepaymentLetterToastPlural).to_string();
                                                    spawn(async move {
                                                        let cfg = CONFIG.read().clone();
                                                        match api::generate_repayment_letters(
                                                            &cfg, phase_id_for_spawn, entry_ids,
                                                        ).await {
                                                            Ok(result) => {
                                                                // Browser-Save: <a download>-Click + revoke_object_url
                                                                // (Pattern aus assembly_details.rs:362-395).
                                                                if let Some(window) = web_sys::window() {
                                                                    if let Some(document) = window.document() {
                                                                        if let Ok(elem) = document.create_element("a") {
                                                                            let _ = elem.set_attribute("href", &result.blob_url);
                                                                            let dl_filename = format!(
                                                                                "auszahlungs_anschreiben_GJ_{}.pdf",
                                                                                fiscal_year_for_spawn
                                                                            );
                                                                            let _ = elem.set_attribute("download", &dl_filename);
                                                                            use wasm_bindgen::JsCast;
                                                                            if let Ok(html_elem) =
                                                                                elem.dyn_into::<web_sys::HtmlElement>()
                                                                            {
                                                                                html_elem.click();
                                                                            }
                                                                        }
                                                                        // T-06-16 mitigation: release blob URL after click.
                                                                        let _ = web_sys::Url::revoke_object_url(&result.blob_url);
                                                                    }
                                                                }
                                                                // D-13-04: Server-Document-Count nutzen (NICHT entry_ids.len()).
                                                                // Singular/Plural-aware Toast (deutsche Grammatik).
                                                                // CR-01 fix: i18n-Strings sind via move-capture aus
                                                                // dem Component-Top-Level uebergeben (KEIN use_i18n()
                                                                // in der async spawn-Closure — Dioxus-Hook-Rules).
                                                                let toast_msg = if result.document_count == 1 {
                                                                    toast_singular
                                                                } else {
                                                                    let count_str = result.document_count.to_string();
                                                                    toast_plural_template.replace("{count}", &count_str)
                                                                };
                                                                show_toast(&mut toast_messages, &mut toast_counter, toast_msg);

                                                                // D-13-09 Selection-Preservation:
                                                                // selected_ids wird hier bewusst NICHT modifiziert. Der
                                                                // Vorstand kann sofort den Phase-8-Batch-Button
                                                                // "Als angeschrieben markieren" mit derselben Selektion
                                                                // ausloesen. Backend toggelt den Status NICHT
                                                                // automatisch — symmetrisch zu Phase 10 (Mail).
                                                            }
                                                            Err(e) => {
                                                                show_toast(&mut toast_messages, &mut toast_counter, e.message);
                                                            }
                                                        }
                                                    });
                                                },
                                                on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                            }
                                        },
                                    },
                                    "export" => match status_value {
                                        RepaymentPhaseStatusTO::Preparation => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "{i18n.t(Key::RepaymentExportNotOpenYet)}"
                                            }
                                        },
                                        // Plan 12-14: ExportTab wired-up.
                                        // D-08: Closed-Status zeigt Export weiterhin (Backend Phase 11 EXPO-01).
                                        _ => rsx! { ExportTab { phase: phase_for_export } },
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
            // Plan 12-09: Add-Entry-Modal-Mount (UI-04). Modal-Wrapper wird nur
            // dann gerendert, wenn show_add_modal.read() == true. on_created
            // inkrementiert entries_reload_trigger (verbatim Counter-Bump) und
            // ruft zusaetzlich load_phase() damit version + opened_at frisch sind
            // (Pitfall #7: Mutation invalidiert lokale State-Annahmen).
            if *show_add_modal.read() {
                Modal {
                    RepaymentEntryAddModal {
                        phase_id,
                        on_close: move |_| show_add_modal.set(false),
                        on_created: move |_| {
                            show_add_modal.set(false);
                            // ── Plan 12-09 verbatim: Counter-Trigger statt load_phase-Cascade ──
                            //
                            // Warum nicht nur load_phase()? load_phase() schreibt nur das
                            // phase-Signal neu — die RepaymentEntryList hat aber `entries`
                            // lokal und sein `use_effect` ist an `reload_trigger` (und
                            // phase-Aenderung) gebunden. Ein Counter-Bump garantiert das
                            // Re-Fetch deterministisch.
                            //
                            // Zusaetzlich load_phase() rufen — damit version + opened_at
                            // etc. aktuell sind.
                            let current = *entries_reload_trigger.read();
                            entries_reload_trigger.set(current.wrapping_add(1));
                            load_phase();
                        },
                        on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                    }
                }
            }
            // Plan 12-10: PaidOut-Bulk-Confirm-Modal (UI-05).
            //
            // Modal-Mount nur, wenn paidout_modal_entries.is_some() UND die Phase
            // geladen ist (share_value wird zum Berechnen der Summe gebraucht).
            // on_complete bekommt (success, failure) und formuliert den Summary-Toast
            // hier in der Detail-Page (Caller-Discretion). Danach Counter-Bump fuer
            // die RepaymentEntryList + load_phase(), damit Status-Spalten aktualisiert
            // werden (PaidOut-Status + ggf. neue version-UUIDs).
            if let Some(entries_to_confirm) = paidout_modal_entries.read().clone() {
                if let Some(p) = phase.read().clone() {
                    Modal {
                        RepaymentEntryPaidOutConfirm {
                            entries: entries_to_confirm,
                            share_value_cents: p.share_value,
                            on_close: move |_| paidout_modal_entries.set(None),
                            on_complete: move |(success, failure): (usize, usize)| {
                                paidout_modal_entries.set(None);
                                let total = success + failure;
                                // D-15 Summary-Toast (deutsch) — Caller-Discretion ueber Wording.
                                let msg = if failure == 0 {
                                    format!("{success} Eintraege als ausbezahlt markiert.")
                                } else {
                                    format!(
                                        "{success} von {total} erfolgreich, {failure} fehlgeschlagen — siehe Status-Spalte.",
                                    )
                                };
                                show_toast(&mut toast_messages, &mut toast_counter, msg);
                                // Counter-Bump + Phase-Reload (Cascade-Effekte sichtbar machen).
                                let current = *entries_reload_trigger.read();
                                entries_reload_trigger.set(current.wrapping_add(1));
                                load_phase();
                            },
                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                        }
                    }
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

    // Plan 12-06: share_value-Inline-Edit (D-05 PHAS-04) — 3 Render-Modi
    let mut editing_share_value = use_signal(|| false);
    let initial_input = format!("{:.2}", (phase.share_value as f64) / 100.0).replace('.', ",");
    let mut share_value_input = use_signal(move || initial_input.clone());
    let mut saving = use_signal(|| false);
    let editable = is_share_value_editable(phase_status);
    let show_audit_hint = matches!(phase_status, RepaymentPhaseStatusTO::Open);
    let phase_version = phase.version;
    let phase_fiscal_year_for_save = phase.fiscal_year;
    let phase_share_value_for_reset = phase.share_value;

    rsx! {
        div { class: "flex flex-col gap-6",
            // Stamm-Daten-Block (read-only fiscal_year + 3-Modi share_value-Edit)
            div { class: "grid grid-cols-2 gap-4",
                div {
                    span { class: "text-sm text-gray-500", "{i18n.t(Key::RepaymentPhaseFiscalYear)}" }
                    p { class: "text-lg font-semibold", "{phase.fiscal_year}" }
                }
                // D-05: 3-Modi-Render fuer share_value
                div {
                    span { class: "text-sm text-gray-500", "{i18n.t(Key::RepaymentPhaseShareValue)}" }
                    if *editing_share_value.read() && editable {
                        div { class: "flex flex-col gap-1",
                            if show_audit_hint {
                                p { class: "text-xs text-orange-700",
                                    "{i18n.t(Key::RepaymentPhaseShareValueEditHint)}"
                                }
                            }
                            div { class: "flex items-center gap-2",
                                input {
                                    class: "border border-gray-300 rounded px-3 py-2 w-32",
                                    r#type: "text",
                                    value: "{share_value_input}",
                                    oninput: move |e| share_value_input.set(e.value()),
                                }
                                span { class: "text-gray-700", "EUR" }
                                button {
                                    r#type: "button",
                                    class: "bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded disabled:opacity-50 min-h-[44px]",
                                    disabled: *saving.read(),
                                    onclick: move |_| {
                                        // Plan 12-02 Kanonik: parse_euro_to_cents (kein lokales Re-Define)
                                        let cents = match parse_euro_to_cents(&share_value_input.read()) {
                                            Some(c) => c,
                                            None => {
                                                on_error.call(
                                                    "Bitte gueltigen Wert > 0 angeben (z.B. 60,00)".into(),
                                                );
                                                return;
                                            }
                                        };
                                        let version = match phase_version {
                                            Some(v) => v,
                                            None => {
                                                on_error.call(
                                                    "Phase hat keine Version — bitte neu laden".into(),
                                                );
                                                return;
                                            }
                                        };
                                        saving.set(true);
                                        let req = crate::api::UpdateRepaymentPhaseRequest {
                                            fiscal_year: phase_fiscal_year_for_save,
                                            share_value: cents,
                                            version,
                                        };
                                        spawn(async move {
                                            let config = CONFIG.read().clone();
                                            match api::update_repayment_phase(&config, phase_id, &req).await {
                                                Ok(_) => {
                                                    editing_share_value.set(false);
                                                    // Pitfall #7: re-fetch statt Response-Version verwenden
                                                    on_changed.call(());
                                                }
                                                Err(e) if e.status == Some(409) => {
                                                    on_error.call(
                                                        "Konflikt — Daten wurden zwischenzeitlich geaendert, bitte erneut speichern".into(),
                                                    );
                                                    editing_share_value.set(false);
                                                    on_changed.call(()); // Reload mit neuer Version
                                                }
                                                Err(e) => on_error.call(e.message),
                                            }
                                            saving.set(false);
                                        });
                                    },
                                    "{i18n.t(Key::Save)}"
                                }
                                button {
                                    r#type: "button",
                                    class: "px-3 py-2 text-gray-700 hover:bg-gray-100 rounded min-h-[44px]",
                                    onclick: move |_| {
                                        // Reset to original value
                                        share_value_input.set(
                                            format!("{:.2}", (phase_share_value_for_reset as f64) / 100.0).replace('.', ","),
                                        );
                                        editing_share_value.set(false);
                                    },
                                    "{i18n.t(Key::Cancel)}"
                                }
                            }
                        }
                    } else {
                        div { class: "flex items-center gap-2",
                            p { class: "text-lg font-semibold", "{format_payout_eur(1, phase.share_value)}" }
                            if editable {
                                button {
                                    r#type: "button",
                                    class: "text-blue-600 hover:text-blue-800 text-sm underline",
                                    onclick: move |_| editing_share_value.set(true),
                                    "{i18n.t(Key::Edit)}"
                                }
                            }
                        }
                    }
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

/// Plan 12-14 UI-02 (EXPO-01..03) — Export-Tab.
///
/// Drei Radio-Buttons fuer `?include=open|all|paid` (Default: open — Phase 11 D-03)
/// + grosser blauer Download-Anker, der das PDF im neuen Tab oeffnet. Der Browser
/// haendelt den Download via Content-Disposition (Backend Phase 11 setzt den Header).
///
/// D-26: Download ist ein `<a>`-Element mit Button-Styling — KEIN `<button>` mit
/// fake-href. D-01 Grep-Gate ist nicht betroffen, weil Radio-Inputs und der
/// Download-Anker keine `<button>`-Tags sind.
///
/// D-08: Bei Status=Closed wird ExportTab im `_ =>`-Branch gerendert (Caller-Site).
/// Backend laesst Export fuer Open UND Closed zu; Filter::Open in Closed-Phase
/// kann eine leere Liste liefern (akzeptables Backend-Verhalten).
#[component]
fn ExportTab(phase: RepaymentPhaseTO) -> Element {
    let i18n = use_i18n();
    let mut selected_include = use_signal(|| ExportInclude::Open);
    let config = CONFIG.read();
    let backend = config.backend.to_string();
    let phase_id = phase.id;
    let download_url = build_export_url(phase_id, *selected_include.read(), &backend);

    rsx! {
        div { class: "flex flex-col gap-4 max-w-xl",
            div {
                h3 { class: "text-lg font-semibold mb-2",
                    "{i18n.t(Key::RepaymentExportInclude)}"
                }
                div { class: "flex flex-col gap-2",
                    ExportIncludeRadio {
                        value: ExportInclude::Open,
                        selected: *selected_include.read(),
                        label: i18n.t(Key::RepaymentExportIncludeOpen).to_string(),
                        on_select: move |v| selected_include.set(v),
                    }
                    ExportIncludeRadio {
                        value: ExportInclude::All,
                        selected: *selected_include.read(),
                        label: i18n.t(Key::RepaymentExportIncludeAll).to_string(),
                        on_select: move |v| selected_include.set(v),
                    }
                    ExportIncludeRadio {
                        value: ExportInclude::Paid,
                        selected: *selected_include.read(),
                        label: i18n.t(Key::RepaymentExportIncludePaid).to_string(),
                        on_select: move |v| selected_include.set(v),
                    }
                }
            }

            // D-26 + D-01: Download via <a>-Element (kein button) mit Button-Styling.
            // target=_blank oeffnet PDF im neuen Tab; Content-Disposition triggert Browser-Download.
            // Plan-Discretion: Falls UAT lieber direkt im selben Tab herunterladen will,
            // kann target weglassen (siehe Summary).
            a {
                class: "inline-block bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded text-center font-semibold min-h-[44px]",
                href: "{download_url}",
                target: "_blank",
                rel: "noopener noreferrer",
                "{i18n.t(Key::RepaymentExportDownload)}"
            }
        }
    }
}

/// Plan 12-14: Einzelner Radio-Button fuer ExportInclude.
///
/// Reine Anzeige-Component — keine Logik im Render-Pfad, nur EventHandler-Forward.
/// Wird in `ExportTab` dreimal verwendet (Open/All/Paid).
#[component]
fn ExportIncludeRadio(
    value: ExportInclude,
    selected: ExportInclude,
    label: String,
    on_select: EventHandler<ExportInclude>,
) -> Element {
    let is_selected = value == selected;
    rsx! {
        label { class: "flex items-center gap-2 cursor-pointer min-h-[44px]",
            input {
                r#type: "radio",
                name: "export_include",
                checked: is_selected,
                onchange: move |_| on_select.call(value),
            }
            span { class: "text-gray-700", "{label}" }
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
        assert!(!should_show_close_button(
            RepaymentPhaseStatusTO::Preparation
        ));
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
        let body =
            r#"{"error":"pending entries","pending_count":2,"pending_member_numbers":["42","43"]}"#;
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

    // Plan 12-13 D-18: build_mail_redirect_url Pure-Func Tests
    #[test]
    fn build_url_with_empty_members() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let url = build_mail_redirect_url(pid, &[]);
        assert!(url.starts_with("/mail?from=repayment&phase_id="));
        assert!(!url.contains("members="));
    }

    #[test]
    fn build_url_with_single_member() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let mid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        let url = build_mail_redirect_url(pid, &[mid]);
        assert!(url.contains("members=550e8400-e29b-41d4-a716-446655440001"));
    }

    #[test]
    fn build_url_with_multiple_members() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let m1 = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        let m2 = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
        let url = build_mail_redirect_url(pid, &[m1, m2]);
        assert!(url.contains("members="));
        // Komma-getrennte UUIDs
        assert!(url
            .contains("550e8400-e29b-41d4-a716-446655440001,550e8400-e29b-41d4-a716-446655440002"));
    }

    #[test]
    fn build_url_starts_with_mail_path() {
        let pid = Uuid::new_v4();
        let url = build_mail_redirect_url(pid, &[]);
        assert!(url.starts_with("/mail?"));
    }

    // Plan 12-14 UI-02 / EXPO-01..03: build_export_url Pure-Func Tests
    // RED-Phase: failing tests for the PDF-Export URL builder.
    // ExportInclude::Open|All|Paid select the ?include= filter (Phase 11 D-03 default open).

    #[test]
    fn build_url_open() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let url = build_export_url(pid, ExportInclude::Open, "https://api.example.com");
        assert_eq!(
            url,
            "https://api.example.com/api/repayment-phase/550e8400-e29b-41d4-a716-446655440000/export/pdf?include=open"
        );
    }

    #[test]
    fn build_url_all() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let url = build_export_url(pid, ExportInclude::All, "https://api.example.com");
        assert!(url.ends_with("include=all"));
    }

    #[test]
    fn build_url_paid() {
        let pid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let url = build_export_url(pid, ExportInclude::Paid, "https://api.example.com");
        assert!(url.ends_with("include=paid"));
    }

    #[test]
    fn build_url_trims_backend_trailing_slash() {
        let pid = Uuid::new_v4();
        let url = build_export_url(pid, ExportInclude::Open, "https://api.example.com/");
        assert!(!url.contains("//api/"));
    }
}
