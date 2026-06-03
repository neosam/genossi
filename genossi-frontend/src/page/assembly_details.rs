//! AssemblyDetails 3-tab page (Phase 4 Plan 08) — admin-only.
//!
//! 3 Tabs (Stamm-Daten / Helfer-Tokens / Anwesenheit). The Anwesenheit-Tab
//! re-uses the EXACT components from /helper/attendance — ATTN-06 Component-First.
//!
//! W-04: BasicsTab + TokenRow + CreateTokenForm sind in Plan 06 als eigene Components
//! extrahiert. Plan 08 importiert sie via `crate::component::*` und ruft sie auf —
//! KEINE inline-Definitionen. Der einzige inline-helper hier ist `TokensTab`, weil er
//! als Page-spezifischer Tab-Body-Wrapper Liste + Create-Modal koordiniert.
//!
//! SYNC-01 wiring: AttendanceList exposes `on_toggle: EventHandler<AttendanceToggleRequest>`
//! (the list is dumb, the page is smart — that's the ATTN-06 reuse anchor). The page
//! handles the toggle by calling `mark_present` / `mark_absent` based on the request's
//! current state; on 200 OK it bumps `refresh_signal` so the list re-fetches authoritative
//! state. Plan 04-09's helper_attendance.rs uses the SAME shell with the SAME wiring.

use dioxus::prelude::*;
use std::str::FromStr;
use uuid::Uuid;
use wasm_bindgen::JsCast;

use crate::api::{self, AssemblyStatusTO, AssemblyTO, HelperTokenCreateResponseTO, HelperTokenTO};
use crate::auth::RequirePrivilege;
use crate::component::{
    show_toast, AssemblyStatusBadge, AttendanceList, AttendanceSearch, AttendanceToggleRequest,
    BasicsTab, ConnState, ConnectionBanner, CreateTokenForm, LiveCounter, Modal, QrCard, TabDef,
    TabStrip, ToastContainer, TokenRow, TopBar,
};
use crate::i18n::{use_i18n, Key};
use crate::page::access_denied::AccessDeniedPage;
use crate::service::config::CONFIG;

#[component]
pub fn AssemblyDetails(id: String) -> Element {
    let i18n = use_i18n();
    let assembly_id = match Uuid::from_str(&id) {
        Ok(u) => u,
        Err(_) => return rsx! { div { class: "p-4 text-red-600", "Invalid assembly id" } },
    };

    let mut assembly = use_signal(|| Option::<AssemblyTO>::None);
    let mut loading = use_signal(|| true);
    let mut active_tab = use_signal(|| "basics".to_string());
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);
    let mut conn_lost = use_signal(|| false);
    // search_query is owned by the page so it can be passed as ReadOnlySignal to AttendanceList.
    let search_query = use_signal(String::new);
    // refresh_signal is bumped by the page after a successful toggle (SYNC-01 acceptance).
    let mut refresh_signal = use_signal(|| 0u64);

    let load = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::get_assembly(&config, assembly_id).await {
                Ok(a) => assembly.set(Some(a)),
                Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    rsx! {
        RequirePrivilege {
            privilege: "admin",
            fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
            TopBar {}
            div { class: "container mx-auto px-4 py-6",
                if *loading.read() {
                    p { class: "text-gray-500", "{i18n.t(Key::Loading)}" }
                } else if let Some(a) = assembly.read().clone() {
                    // Header
                    div { class: "flex items-center justify-between mb-4",
                        h1 { class: "text-2xl font-bold", "{a.name}" }
                        AssemblyStatusBadge { status: a.status.clone() }
                    }
                    // Tabs
                    {
                        // D-19: 4th tab "Export" appears ONLY when the assembly is closed.
                        let mut tab_defs = vec![
                            TabDef { key: "basics", label: i18n.t(Key::AssemblyTabBasics).to_string() },
                            TabDef { key: "tokens", label: i18n.t(Key::AssemblyTabTokens).to_string() },
                            TabDef { key: "attendance", label: i18n.t(Key::AssemblyTabAttendance).to_string() },
                        ];
                        if matches!(a.status, AssemblyStatusTO::Closed) {
                            tab_defs.push(TabDef {
                                key: "export",
                                label: i18n.t(Key::AssemblyTabExport).to_string(),
                            });
                        }
                        let active_key = active_tab.read().clone();
                        let assembly_status = a.status.clone();
                        let assembly_for_basics = a.clone();
                        let assembly_for_export = a.clone();
                        rsx! {
                            TabStrip {
                                tabs: tab_defs,
                                active_key: active_key.clone(),
                                on_change: move |k: String| active_tab.set(k),
                                // Body branched on active key — DELEGATES to extracted components.
                                match active_key.as_str() {
                                    "basics" => rsx! {
                                        // W-04 + W-01: BasicsTab handles ReadOnly/Edit toggle + open/close confirms.
                                        BasicsTab {
                                            assembly: assembly_for_basics,
                                            on_changed: move |_| load(),
                                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                        }
                                    },
                                    "tokens" => rsx! {
                                        TokensTab {
                                            assembly_id: assembly_id,
                                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                        }
                                    },
                                    "attendance" => match assembly_status {
                                        AssemblyStatusTO::Preparation => rsx! {
                                            div { class: "text-center py-12 text-gray-500",
                                                "{i18n.t(Key::AssemblyAttendanceNotOpenYet)}"
                                            }
                                        },
                                        status_otherwise => {
                                            let polling_enabled = matches!(status_otherwise, AssemblyStatusTO::Open);
                                            let read_only = matches!(status_otherwise, AssemblyStatusTO::Closed);
                                            rsx! {
                                                AttendanceTab {
                                                    assembly_id: assembly_id,
                                                    polling_enabled: polling_enabled,
                                                    read_only: read_only,
                                                    conn_lost: conn_lost,
                                                    search_query: search_query,
                                                    refresh_signal: refresh_signal,
                                                    on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                                }
                                            }
                                        }
                                    },
                                    "export" => rsx! {
                                        // D-19 + D-20: ExportTab is inline in this file (no separate component file).
                                        // Visibility is gated upstream by the tab_defs push above.
                                        ExportTab {
                                            assembly: assembly_for_export,
                                            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
                                        }
                                    },
                                    _ => rsx! {},
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

// ─── AttendanceTab (Page-internal smart wrapper) ────────────────────────
// Wraps the four shared components used by /helper/attendance and owns the
// SMART toggle wiring: receives `AttendanceToggleRequest` from `AttendanceList`,
// calls `mark_present` / `mark_absent`, and bumps `refresh_signal` on success
// (SYNC-01 acceptance). Plan 04-09's helper_attendance.rs reproduces this
// wiring — that's the ATTN-06 reuse anchor.
#[component]
fn AttendanceTab(
    assembly_id: Uuid,
    polling_enabled: bool,
    read_only: bool,
    conn_lost: Signal<bool>,
    search_query: Signal<String>,
    refresh_signal: Signal<u64>,
    on_error: EventHandler<String>,
) -> Element {
    let mut conn_lost = conn_lost;
    let mut search_query = search_query;
    let mut refresh_signal = refresh_signal;

    rsx! {
        ConnectionBanner { visible: *conn_lost.read() }
        LiveCounter {
            assembly_id: assembly_id,
            polling_enabled: polling_enabled,
            on_connection_state: move |s: ConnState| {
                conn_lost.set(matches!(s, ConnState::Lost));
            },
        }
        AttendanceSearch {
            on_change: move |q: String| search_query.set(q),
        }
        AttendanceList {
            assembly_id: assembly_id,
            search_query: search_query,
            read_only: read_only,
            refresh_signal: refresh_signal,
            // Smart parent wires the toggle: call API, then bump refresh_signal on success.
            on_toggle: move |req: AttendanceToggleRequest| {
                let aid = assembly_id;
                spawn(async move {
                    let config = CONFIG.read().clone();
                    let result = if req.current_is_present {
                        api::mark_absent(&config, aid, req.member_id).await
                    } else {
                        api::mark_present(&config, aid, req.member_id).await
                    };
                    match result {
                        Ok(_) => {
                            // SYNC-01: bump on 200 OK so the list re-fetches authoritative state.
                            refresh_signal.with_mut(|n| *n += 1);
                        }
                        Err(e) => on_error.call(e.message),
                    }
                });
            },
        }
    }
}

// ─── TokensTab (Page-internal, NOT extracted) ──────────────────────────
// TokensTab orchestriert die Token-Liste + Create-Modal + Just-Created-QrCard.
// Sie nutzt die extrahierten W-04 Components TokenRow + CreateTokenForm.
// Inline bleibt sie weil sie page-spezifischen state hält (`just_created` —
// One-Time-Show via Phase 2 D-21).
#[component]
fn TokensTab(assembly_id: Uuid, on_error: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut tokens = use_signal(Vec::<HelperTokenTO>::new);
    let mut loading = use_signal(|| true);
    let mut show_create = use_signal(|| false);
    let mut just_created = use_signal(|| Option::<HelperTokenCreateResponseTO>::None);

    let load = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_helper_tokens(&config, assembly_id).await {
                Ok(list) => tokens.set(list),
                Err(e) => on_error.call(e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    rsx! {
        div { class: "flex justify-between items-start mb-4",
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::HelperTokens)}" }
            button {
                r#type: "button",
                class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded min-h-[44px]",
                onclick: move |_| show_create.set(true),
                "{i18n.t(Key::HelperTokenCreate)}"
            }
        }

        // Just-created card (One-Time-Show — Phase 2 D-21)
        if let Some(resp) = just_created.read().clone() {
            div { class: "mb-6",
                QrCard {
                    memo: resp.token.memo.clone(),
                    code: resp.code.clone(),
                    qr_svg: resp.qr_svg.clone(),
                }
                div { class: "flex justify-center mt-2",
                    button {
                        r#type: "button",
                        class: "text-sm text-gray-600 underline min-h-[44px] px-4",
                        onclick: move |_| just_created.set(None),
                        "{i18n.t(Key::Close)}"
                    }
                }
            }
        }

        if *loading.read() {
            p { "{i18n.t(Key::Loading)}" }
        } else if tokens.read().is_empty() {
            div { class: "text-center py-12",
                p { class: "text-lg font-medium text-gray-700", "{i18n.t(Key::HelperTokenEmpty)}" }
                p { class: "text-sm text-gray-500 mt-2", "{i18n.t(Key::HelperTokenEmptyHint)}" }
            }
        } else {
            div { class: "flex flex-col gap-2",
                for t in tokens.read().iter() {
                    // W-04: TokenRow is extracted to Plan 06 (crate::component::TokenRow).
                    TokenRow {
                        key: "{t.id}",
                        token: t.clone(),
                        assembly_id: assembly_id,
                        on_changed: move |_| load(),
                        on_error: on_error,
                    }
                }
            }
        }

        if *show_create.read() {
            Modal {
                // W-04: CreateTokenForm is extracted to Plan 06 (crate::component::CreateTokenForm).
                CreateTokenForm {
                    assembly_id: assembly_id,
                    on_close: move |_| show_create.set(false),
                    on_created: move |resp: HelperTokenCreateResponseTO| {
                        show_create.set(false);
                        just_created.set(Some(resp));
                        load();
                    },
                    on_error: on_error,
                }
            }
        }
    }
}

// ─── ExportTab (Page-internal, NOT extracted — D-20) ────────────────────
// Phase 6 / Plan 04. The Export-Tab is gated by `assembly.status == Closed`
// (D-11 + D-19) in the caller's `tab_defs.push(...)` above. This component
// renders the Format-Cards (PDF/CSV/XLSX), the Include-RadioGroup (all/present),
// a reactive filename-preview, and a submit button that triggers a blob-URL
// download via the api::export_attendance_url pipeline.
//
// D-20: inline here, NOT in src/component/, because reuse is not foreseeable
// (the deferred ideas Sammelexport / E-Mail-Versand are separate phases).
#[component]
fn ExportTab(assembly: AssemblyTO, on_error: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut selected_format = use_signal(|| "pdf".to_string());
    let mut selected_include = use_signal(|| "all".to_string());
    let mut submitting = use_signal(|| false);

    // D-15 filename preview: gv-{YYYY-MM-DD}-teilnehmer.{ext}.
    // `assembly.date` is the ISO-8601 string from the backend; if parse fails
    // or `None`, fall back to `gv-teilnehmer.{ext}` without the date segment.
    let date_str = format_assembly_date_yyyy_mm_dd(&assembly.date).unwrap_or_default();
    let filename = if date_str.is_empty() {
        format!("gv-teilnehmer.{}", selected_format.read())
    } else {
        format!("gv-{}-teilnehmer.{}", date_str, selected_format.read())
    };

    let assembly_id = assembly.id;
    // Download trigger lives on the button's `onclick`, NOT on `<form onsubmit>`.
    // Project-wide convention since hotfix e245013: action buttons set `r#type: "button"`
    // and bind `onclick` directly. Even `evt.prevent_default()` inside an `onsubmit`
    // closure can still let the page reload in Dioxus 0.6 when the closure spawns
    // async work — the click-based path sidesteps that class of bug entirely.
    let on_submit = move |_evt: MouseEvent| {
        if *submitting.read() {
            return;
        }
        let fmt = selected_format.read().clone();
        let inc = selected_include.read().clone();
        let date_for_dl = date_str.clone();
        submitting.set(true);
        spawn(async move {
            let cfg = CONFIG.read().clone();
            match api::export_attendance_url(&cfg, assembly_id, &fmt, &inc).await {
                Ok(blob_url) => {
                    // Programmatic <a download> click — see UI-SPEC §Success Path.
                    // We avoid the HtmlAnchorElement feature dependency by using
                    // generic Element / HtmlElement traits + set_attribute.
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Ok(elem) = document.create_element("a") {
                                let _ = elem.set_attribute("href", &blob_url);
                                let dl_filename = if date_for_dl.is_empty() {
                                    format!("gv-teilnehmer.{}", fmt)
                                } else {
                                    format!("gv-{}-teilnehmer.{}", date_for_dl, fmt)
                                };
                                let _ = elem.set_attribute("download", &dl_filename);
                                if let Ok(html_elem) = elem.dyn_into::<web_sys::HtmlElement>() {
                                    html_elem.click();
                                }
                            }
                            // T-06-16 mitigation: release the blob URL after click.
                            let _ = web_sys::Url::revoke_object_url(&blob_url);
                        }
                    }
                }
                Err(e) => {
                    // AppError.status is Option<u16> (api.rs:16).
                    let key = match e.status {
                        Some(409) => Key::AttendanceExportError409,
                        Some(403) => Key::AttendanceExportError403,
                        _ => Key::AttendanceExportErrorNetwork,
                    };
                    // Re-acquire i18n inside the spawned task: I18N is a global signal,
                    // so a fresh `use_i18n()` clone avoids capturing the outer-scope I18n
                    // value (which would make this closure FnOnce — Dioxus expects FnMut
                    // on form onsubmit handlers).
                    let i18n_inner = use_i18n();
                    on_error.call(i18n_inner.t(key).to_string());
                }
            }
            submitting.set(false);
        });
    };

    rsx! {
        div { class: "bg-white p-4 sm:p-6 rounded-lg border border-gray-200",
            h2 { class: "text-xl font-semibold mb-2", "{i18n.t(Key::AttendanceExportHeading)}" }
            p { class: "text-sm text-gray-600 mb-6", "{i18n.t(Key::AttendanceExportSubheading)}" }

            div { class: "flex flex-col gap-6",

                // ===== Format Radio-Cards =====
                div {
                    span { class: "text-sm text-gray-700 mb-2 block",
                        "{i18n.t(Key::AttendanceExportFormatLabel)}"
                    }
                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
                        for entry in [
                            ("pdf",  Key::AttendanceExportFormatPdfTitle,  Key::AttendanceExportFormatPdfHint),
                            ("csv",  Key::AttendanceExportFormatCsvTitle,  Key::AttendanceExportFormatCsvHint),
                            ("xlsx", Key::AttendanceExportFormatXlsxTitle, Key::AttendanceExportFormatXlsxHint),
                        ].into_iter() {
                            {
                                let (fmt_key, title_key, hint_key) = entry;
                                // Resolve i18n strings before rsx! (Key is not Copy, and rsx!
                                // produces a FnOnce closure that would otherwise move the keys).
                                let title_text = i18n.t(title_key).to_string();
                                let hint_text = i18n.t(hint_key).to_string();
                                let is_selected = selected_format.read().as_str() == fmt_key;
                                let card_class = if is_selected {
                                    "border-2 border-blue-500 bg-blue-50 px-4 py-3 rounded cursor-pointer flex flex-col gap-1 min-h-[44px]"
                                } else {
                                    "border-2 border-gray-200 hover:border-gray-300 bg-white px-4 py-3 rounded cursor-pointer flex flex-col gap-1 min-h-[44px]"
                                };
                                let value_for_click = fmt_key.to_string();
                                rsx! {
                                    label {
                                        key: "{fmt_key}",
                                        class: "{card_class}",
                                        onclick: move |_| selected_format.set(value_for_click.clone()),
                                        input { r#type: "radio", name: "export_format", value: "{fmt_key}",
                                                class: "sr-only", checked: is_selected }
                                        span { class: "text-base font-semibold", "{title_text}" }
                                        span { class: "text-xs text-gray-500", "{hint_text}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // ===== Include Radio-Group =====
                div {
                    span { class: "text-sm text-gray-700 mb-2 block",
                        "{i18n.t(Key::AttendanceExportIncludeLabel)}"
                    }
                    div { class: "flex flex-col gap-2",
                        for entry in [
                            ("all",     Key::AttendanceExportIncludeAll),
                            ("present", Key::AttendanceExportIncludePresent),
                        ].into_iter() {
                            {
                                let (inc_key, label_key) = entry;
                                let label_text = i18n.t(label_key).to_string();
                                let val = inc_key.to_string();
                                let is_sel = selected_include.read().as_str() == inc_key;
                                rsx! {
                                    label {
                                        key: "{inc_key}",
                                        class: "flex items-center gap-2 cursor-pointer min-h-[44px] px-2",
                                        input {
                                            r#type: "radio",
                                            name: "export_include",
                                            value: "{inc_key}",
                                            checked: is_sel,
                                            onclick: move |_| selected_include.set(val.clone()),
                                        }
                                        span { class: "text-base", "{label_text}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // ===== Filename Preview =====
                div { class: "flex items-baseline gap-3 text-sm",
                    span { class: "text-gray-700", "{i18n.t(Key::AttendanceExportFilenameLabel)}" }
                    code { class: "text-xs text-gray-500 font-mono", "{filename}" }
                }

                // ===== Submit Button =====
                div { class: "flex justify-end",
                    button {
                        r#type: "button",
                        onclick: on_submit,
                        class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded min-h-[44px] disabled:opacity-50",
                        disabled: *submitting.read(),
                        if *submitting.read() {
                            "{i18n.t(Key::AttendanceExportSubmitLoading)}"
                        } else {
                            "{i18n.t(Key::AttendanceExportSubmit)}"
                        }
                    }
                }
            }
        }
    }
}

// Pure helper — testable independently. Parses YYYY-MM-DD out of an ISO-8601
// timestamp string ("2026-05-15T19:00:00Z" -> Some("2026-05-15")). Returns
// `None` for `None`, empty, or malformed input.
fn format_assembly_date_yyyy_mm_dd(date: &Option<String>) -> Option<String> {
    let s = date.as_ref()?;
    let date_part = s.split('T').next()?;
    // Basic shape validation: 10 chars, "-" at positions 4 and 7.
    if date_part.len() != 10
        || date_part.chars().nth(4) != Some('-')
        || date_part.chars().nth(7) != Some('-')
    {
        return None;
    }
    Some(date_part.to_string())
}

#[cfg(test)]
mod export_tab_tests {
    use super::format_assembly_date_yyyy_mm_dd;

    #[test]
    fn format_date_extracts_yyyy_mm_dd_from_iso8601() {
        assert_eq!(
            format_assembly_date_yyyy_mm_dd(&Some("2026-05-15T19:00:00Z".to_string())),
            Some("2026-05-15".to_string())
        );
    }

    #[test]
    fn format_date_returns_none_for_invalid_input() {
        assert_eq!(format_assembly_date_yyyy_mm_dd(&None), None);
        assert_eq!(
            format_assembly_date_yyyy_mm_dd(&Some("invalid".to_string())),
            None
        );
        assert_eq!(
            format_assembly_date_yyyy_mm_dd(&Some("2026/05/15".to_string())),
            None
        );
    }

    #[test]
    fn format_date_extracts_when_only_date_present() {
        // Defensive: backend sometimes returns just "YYYY-MM-DD" without time.
        assert_eq!(
            format_assembly_date_yyyy_mm_dd(&Some("2026-05-15".to_string())),
            Some("2026-05-15".to_string())
        );
    }
}
