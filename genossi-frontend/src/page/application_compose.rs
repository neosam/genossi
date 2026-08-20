//! Phase 32 (APUI-01/APUI-02, D-01..D-05): Dedizierte Application-Mail-Compose-
//! Vollseite.
//!
//! `component/application_detail.rs` ist selbst ein `Modal` mit verschachtelten
//! Confirm/Reject-Modals — eine Compose-Oberflaeche darin waere Modal-in-Modal
//! (Anti-Pattern, D-01). Deshalb eine eigene Route/Seite, 1:1 nach dem
//! `mail_page.rs`-Vorbild aus den bestehenden `mail_compose/*`-Bausteinen
//! zusammengesetzt (Component-First, kein geforktes UI, APUI-02).
//!
//! Interaktions-Kern:
//! - TemplateSelector auf Antragsteller-Vorlagen gefiltert und mit der
//!   „Zahlungserinnerung" (Seed …0003) vorbefuellt (D-03).
//! - Debounced Live-Vorschau ueber `preview_application_mail` (Backend-Render-
//!   Kernel, NICHT die member-scoped `preview_mail`); die aufgeloeste Vorschau
//!   IST die Bestaetigung (D-04/D-05), waehrend Pending bleibt die letzte
//!   Vorschau stehen (kein Flackern).
//! - Prominente „zuletzt gesendet"-Zeile als Anti-Doppelversand-Guard (D-06).
//! - Senden-Trigger als `div`/`button` mit `onclick` + `r#type: "button"` (KEIN
//!   `form onsubmit` — Reload-Falle, Vorbild `repayment_phases.rs`); `disabled`
//!   waehrend des Requests UND bei leerem Betreff (kein Doppelversand, D-05).
//! - Erfolg → Success-Toast + Rueckkehr zur Antragsliste.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, ApplicationPreviewResponse};
use crate::auth::{RequirePrivilege, PRIVILEGE_ADMIN};
use crate::component::mail_compose::mail_preview_frame::{
    preview_srcdoc, MailPreviewFrame, PreviewMode,
};
use crate::component::mail_compose::{plain_to_html, MailSubjectInput, TemplateSelector, WysiwygEditor};
use crate::component::{show_toast, ErrorAlert, ToastContainer, TopBar};
use crate::i18n::{use_i18n, Key};
use crate::page::AccessDeniedPage;
use crate::router::Route;
use crate::service::config::CONFIG;

/// Feste UUID der deutschen Antragsteller-Vorlage „Zahlungserinnerung"
/// (Seed-Migration `20260820000001_seed_zahlungserinnerung_template.sql`,
/// `template_type = 'application'`). Wird beim Mount vorausgewaehlt (D-03);
/// faellt sie einmal weg, greift der Fallback auf die erste Antragsteller-
/// Vorlage.
const DEFAULT_APPLICATION_TEMPLATE_ID: &str = "00000000-0000-0000-0000-000000000003";
const APPLICATION_TEMPLATE_TYPE: &str = "application";

/// Debounce-Fenster fuer die Live-Vorschau (D-04, Discretion): lang genug, dass
/// Tippen keinen Request pro Anschlag ausloest, kurz genug fuer „lebendige"
/// Vorschau.
const PREVIEW_DEBOUNCE_MS: u32 = 400;

/// Debounced Trigger fuer die aufgeloeste Application-Mail-Vorschau (D-04).
///
/// `my_gen` ist die Generation dieses Laufs; erhoeht die Call-Site sie durch
/// eine neuere Eingabe, verwirft der abgelaufene Lauf sein Ergebnis stillbar —
/// dadurch bleibt die zuletzt aufgeloeste Vorschau stehen (kein Flackern) und
/// es „gewinnt" immer die neueste Eingabe.
fn schedule_preview(
    id: Uuid,
    subject: String,
    body: String,
    body_html: String,
    debounce_gen: Signal<u64>,
    my_gen: u64,
    mut preview_result: Signal<Option<ApplicationPreviewResponse>>,
    mut preview_pending: Signal<bool>,
    mut error: Signal<Option<api::AppError>>,
) {
    spawn(async move {
        preview_pending.set(true);
        gloo_timers::future::TimeoutFuture::new(PREVIEW_DEBOUNCE_MS).await;
        // Zwischenzeitlich neuere Eingabe? Diesen Lauf verwerfen — die letzte
        // Vorschau bleibt sichtbar.
        if *debounce_gen.read() != my_gen {
            return;
        }
        let config = CONFIG.read().clone();
        let body_html_opt: Option<&str> = if body_html.trim().is_empty() {
            None
        } else {
            Some(body_html.as_str())
        };
        match api::preview_application_mail(&config, id, &subject, &body, body_html_opt).await {
            Ok(resolved) => {
                // Nur uebernehmen, wenn dies noch der aktuelle Lauf ist.
                if *debounce_gen.read() == my_gen {
                    preview_result.set(Some(resolved));
                    error.set(None);
                }
            }
            Err(e) => {
                if *debounce_gen.read() == my_gen {
                    error.set(Some(e));
                }
            }
        }
        preview_pending.set(false);
    });
}

/// Erhoeht die Debounce-Generation und plant eine neue Vorschau (D-04). Als
/// freie Funktion (statt geteilter Closure), damit jeder Event-Handler sie
/// aufrufen kann, ohne Closure-Copy-Semantik zu bemuehen; alle Signale sind
/// `Copy` und werden by-value hereingereicht.
#[allow(clippy::too_many_arguments)]
fn bump_and_preview(
    parsed_id: Option<Uuid>,
    subject: Signal<String>,
    body: Signal<String>,
    body_html: Signal<String>,
    mut debounce_gen: Signal<u64>,
    preview_result: Signal<Option<ApplicationPreviewResponse>>,
    preview_pending: Signal<bool>,
    error: Signal<Option<api::AppError>>,
) {
    let Some(app_id) = parsed_id else {
        return;
    };
    let g = *debounce_gen.read() + 1;
    debounce_gen.set(g);
    schedule_preview(
        app_id,
        subject.read().clone(),
        body.read().clone(),
        body_html.read().clone(),
        debounce_gen,
        g,
        preview_result,
        preview_pending,
        error,
    );
}

#[component]
pub fn ApplicationCompose(id: String) -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    // Antrags-ID aus dem Routen-Segment. Ungueltige UUID → sofort erkennbar
    // statt stiller Fehl-Requests.
    let parsed_id: Option<Uuid> = Uuid::parse_str(&id).ok();

    let mut subject = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut body_html = use_signal(String::new);
    let mut selected_template_id = use_signal(|| Option::<String>::None);
    let mut sending = use_signal(|| false);

    let mut communications =
        use_signal(Vec::<rest_types::CommunicationEntryTO>::new);
    let mut preview_result = use_signal(|| None::<ApplicationPreviewResponse>);
    let preview_pending = use_signal(|| false);
    let mut error: Signal<Option<api::AppError>> = use_signal(|| None);

    // Debounce-Generation (D-04): jede Eingabe erhoeht sie; abgelaufene
    // Preview-Laeufe erkennen daran, dass sie ueberholt wurden.
    let debounce_gen = use_signal(|| 0u64);

    // Toast-State fuer den Erfolgs-Hinweis nach dem Versand.
    let mut toast_messages = use_signal(Vec::<(u64, String)>::new);
    let mut toast_counter = use_signal(|| 0u64);

    // Mount: Antragsteller-Vorlagen laden, „Zahlungserinnerung" vorbefuellen
    // (D-03), Kommunikations-Historie laden (Anti-Doppelversand-Guard), und
    // eine initiale Vorschau ausloesen.
    use_effect(move || {
        let Some(app_id) = parsed_id else {
            return;
        };
        spawn(async move {
            let config = CONFIG.read().clone();

            // Kommunikations-Historie fuer die „zuletzt gesendet"-Zeile.
            if let Ok(comms) = api::get_application_communications(&config, app_id).await {
                communications.set(comms);
            }

            // Antragsteller-Vorlagen laden und die Default-Vorlage vorbefuellen.
            if let Ok(all) = api::list_mail_templates(&config).await {
                let application_templates =
                    api::filter_templates_by_type(&all, APPLICATION_TEMPLATE_TYPE);
                let default = application_templates
                    .iter()
                    .find(|t| t.id == DEFAULT_APPLICATION_TEMPLATE_ID)
                    .or_else(|| application_templates.first());
                if let Some(tpl) = default {
                    subject.set(tpl.subject.clone());
                    // TemplateSelector liefert sonst Plain-Text; hier direkt aus
                    // der Vorlage vorbefuellen und fuer den WysiwygEditor in HTML
                    // konvertieren (escape + \n→<br>), damit der Editor nicht
                    // leer startet.
                    let tpl_body = tpl.body.clone();
                    body.set(tpl_body.clone());
                    body_html.set(match tpl.body_html.as_ref() {
                        Some(h) if !h.trim().is_empty() => h.clone(),
                        _ => plain_to_html(&tpl_body),
                    });
                    selected_template_id.set(Some(tpl.id.clone()));

                    // Initiale aufgeloeste Vorschau (D-04): die Vorschau ist die
                    // Bestaetigung, also von Anfang an vorhanden.
                    bump_and_preview(
                        Some(app_id),
                        subject,
                        body,
                        body_html,
                        debounce_gen,
                        preview_result,
                        preview_pending,
                        error,
                    );
                }
            }
        });
    });

    // „zuletzt gesendet"-Zeile (D-06): nur ausgehende Eintraege sind fuer den
    // Anti-Doppelversand-Guard relevant; die Historie kommt bereits nach Datum
    // absteigend sortiert (Backend ORDER BY date DESC).
    let last_sent = {
        let comms = communications.read();
        let outbound: Vec<rest_types::CommunicationEntryTO> = comms
            .iter()
            .filter(|e| e.direction == rest_types::CommunicationDirection::Outbound)
            .cloned()
            .collect();
        api::last_outbound_summary(&outbound)
    };

    // Backend-Basis fuer die Asset-Injektion in der HTML-Vorschau.
    let backend = CONFIG.read().backend.clone();

    rsx! {
        RequirePrivilege {
            privilege: PRIVILEGE_ADMIN,
            fallback: rsx! { AccessDeniedPage { required_privilege: PRIVILEGE_ADMIN.to_string() } },
            div { class: "flex flex-col min-h-screen",
                TopBar {}
                div { class: "flex-1 container mx-auto px-4 py-8",
                    h1 { class: "text-3xl font-bold mb-6", {i18n.t(Key::MailCompose)} }

                    if parsed_id.is_none() {
                        div { class: "bg-red-50 border border-red-200 rounded p-4 text-sm text-red-700",
                            "Ungültige Antrags-ID."
                        }
                    } else {
                        // Fehler beim Senden/Preview — nie ein stilles 200 (APMAIL-02).
                        if let Some(ref err) = *error.read() {
                            ErrorAlert {
                                error: err.clone(),
                                on_dismiss: move |_| error.set(None),
                            }
                        }

                        div { class: "bg-white rounded-lg shadow p-6 mb-6",
                            div { class: "space-y-4",
                                MailSubjectInput {
                                    value: subject.read().clone(),
                                    on_change: move |val: String| {
                                        subject.set(val);
                                        bump_and_preview(
                                            parsed_id, subject, body, body_html,
                                            debounce_gen, preview_result, preview_pending, error,
                                        );
                                    },
                                }

                                TemplateSelector {
                                    filter_type: Some(APPLICATION_TEMPLATE_TYPE.to_string()),
                                    initial_template_id: selected_template_id.read().clone(),
                                    on_select: move |template_body: String| {
                                        body.set(template_body.clone());
                                        body_html.set(plain_to_html(&template_body));
                                        bump_and_preview(
                                            parsed_id, subject, body, body_html,
                                            debounce_gen, preview_result, preview_pending, error,
                                        );
                                    },
                                    on_select_id: move |tid: Option<String>| {
                                        selected_template_id.set(tid);
                                    },
                                }

                                // WysiwygEditor als EINZIGE Eingabequelle des
                                // Bodys; `key` erzwingt Remount bei Vorlagen-
                                // Wechsel (der Editor seedet innerHTML nur beim
                                // Mount).
                                {
                                    let editor_key = selected_template_id
                                        .read()
                                        .clone()
                                        .unwrap_or_else(|| "__no_template__".to_string());
                                    rsx! {
                                        WysiwygEditor {
                                            key: "{editor_key}",
                                            value: body_html.read().clone(),
                                            on_change: move |(plain, html): (String, String)| {
                                                body.set(plain);
                                                body_html.set(html);
                                                bump_and_preview(
                                                    parsed_id, subject, body, body_html,
                                                    debounce_gen, preview_result, preview_pending, error,
                                                );
                                            },
                                        }
                                    }
                                }

                                // Fokuspunkt: die aufgeloeste Live-Vorschau (D-05).
                                // Sie IST die Bestaetigung — waehrend Pending bleibt
                                // die letzte Vorschau sichtbar (D-04).
                                div { class: "bg-gray-50 rounded-lg p-4",
                                    div { class: "flex items-center gap-2 mb-2",
                                        h3 { class: "text-sm font-medium text-gray-700",
                                            {i18n.t(Key::MailTemplatePreview)}
                                        }
                                        if *preview_pending.read() {
                                            span { class: "text-xs text-gray-500", "…" }
                                        }
                                    }
                                    if let Some(preview) = preview_result.read().as_ref() {
                                        p { class: "text-sm font-medium text-gray-700 mb-2",
                                            "{i18n.t(Key::MailSubject)}: "
                                            span { class: "font-normal", "{preview.subject}" }
                                        }
                                        {
                                            let resolved_html = match preview.body_html.as_ref() {
                                                Some(h) if !h.trim().is_empty() => {
                                                    crate::component::mail_compose::mail_preview_frame::inject_asset_src(h, &backend)
                                                }
                                                _ => plain_to_html(&preview.body),
                                            };
                                            let srcdoc = preview_srcdoc(&resolved_html);
                                            rsx! {
                                                MailPreviewFrame {
                                                    mode: PreviewMode::Desktop,
                                                    srcdoc,
                                                }
                                            }
                                        }
                                    } else {
                                        p { class: "text-sm text-gray-400 italic",
                                            {i18n.t(Key::Loading)}
                                        }
                                    }
                                }

                                // Anti-Doppelversand-Guard (D-06): „zuletzt
                                // gesendet"-Zeile direkt ueber dem Senden-Button.
                                div { class: "text-sm text-gray-600",
                                    if let Some((subj, status, date)) = last_sent {
                                        {
                                            let status_part = status.unwrap_or_default();
                                            rsx! {
                                                span { class: "font-medium", {i18n.t(Key::LastSentSummary)} ": " }
                                                span { "{subj} — {status_part} ({date})" }
                                            }
                                        }
                                    } else {
                                        span { class: "italic text-gray-400", {i18n.t(Key::NeverSent)} }
                                    }
                                }

                                // Senden-Trigger: KEIN form onsubmit (Reload-Falle),
                                // sondern button+onclick+r#type:"button" (D-05,
                                // Vorbild repayment_phases.rs). disabled waehrend
                                // Request UND bei leerem Betreff.
                                button {
                                    class: "bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50",
                                    r#type: "button",
                                    disabled: *sending.read() || subject.read().is_empty(),
                                    onclick: move |_| {
                                        let Some(app_id) = parsed_id else {
                                            return;
                                        };
                                        let subj = subject.read().clone();
                                        let b = body.read().clone();
                                        let bh_value = body_html.read().clone();
                                        let template_id_owned: Option<String> =
                                            selected_template_id.read().clone();
                                        let i18n = i18n.clone();
                                        spawn(async move {
                                            sending.set(true);
                                            error.set(None);
                                            let config = CONFIG.read().clone();
                                            let body_html_opt: Option<&str> =
                                                if bh_value.trim().is_empty() {
                                                    None
                                                } else {
                                                    Some(bh_value.as_str())
                                                };
                                            let template_id: Option<&str> =
                                                template_id_owned.as_deref();
                                            match api::send_application_mail(
                                                &config,
                                                app_id,
                                                &subj,
                                                &b,
                                                body_html_opt,
                                                template_id,
                                            )
                                            .await
                                            {
                                                Ok(()) => {
                                                    show_toast(
                                                        &mut toast_messages,
                                                        &mut toast_counter,
                                                        i18n.t(Key::MailJobCreated).to_string(),
                                                    );
                                                    nav.push(Route::ApplicationsPage {});
                                                }
                                                Err(e) => {
                                                    error.set(Some(e));
                                                }
                                            }
                                            sending.set(false);
                                        });
                                    },
                                    if *sending.read() {
                                        {i18n.t(Key::MailSending)}
                                    } else {
                                        {i18n.t(Key::MailSend)}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ToastContainer { messages: toast_messages }
        }
    }
}
