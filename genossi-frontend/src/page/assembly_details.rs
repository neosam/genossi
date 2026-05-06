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

use crate::api::{self, AssemblyStatusTO, AssemblyTO, HelperTokenCreateResponseTO, HelperTokenTO};
use crate::auth::RequirePrivilege;
use crate::component::{
    AssemblyStatusBadge, AttendanceList, AttendanceSearch, AttendanceToggleRequest, BasicsTab,
    ConnState, ConnectionBanner, CreateTokenForm, LiveCounter, Modal, QrCard, TabDef, TabStrip,
    ToastContainer, TokenRow, TopBar, show_toast,
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
                        let tab_defs = vec![
                            TabDef { key: "basics", label: i18n.t(Key::AssemblyTabBasics).to_string() },
                            TabDef { key: "tokens", label: i18n.t(Key::AssemblyTabTokens).to_string() },
                            TabDef { key: "attendance", label: i18n.t(Key::AssemblyTabAttendance).to_string() },
                        ];
                        let active_key = active_tab.read().clone();
                        let assembly_status = a.status.clone();
                        let assembly_for_basics = a.clone();
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
