//! HelperAttendance Page (Phase 4 Plan 09) — /helper/attendance route.
//!
//! Wraps in HelperShell (no global app-chrome per D-07 + Datenschutz). Body uses
//! the SAME 4 components as the Vorstand Anwesenheits-Tab in assembly_details.rs
//! — ATTN-06 Component-Reuse anchor.
//!
//! Smart-parent toggle wiring is identical to `AttendanceTab` in
//! `assembly_details.rs`: receives `AttendanceToggleRequest` from
//! `AttendanceList`, calls `mark_present` / `mark_absent`, and bumps
//! `refresh_signal` on success (SYNC-01 acceptance).
//!
//! Auth gate: 401 from `/api/helper/session` → nav to `/helper` (Toast).

use dioxus::prelude::*;

use crate::api::{self, HelperSessionTO};
use crate::component::{
    AttendanceList, AttendanceSearch, AttendanceToggleRequest, ConnState, ConnectionBanner,
    HelperShell, LiveCounter, ToastContainer, show_toast,
};
use crate::i18n::{use_i18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;

#[component]
pub fn HelperAttendance() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    let mut session = use_signal(|| Option::<HelperSessionTO>::None);
    let mut loading = use_signal(|| true);
    let mut conn_lost = use_signal(|| false);
    let mut search_query = use_signal(String::new);
    let mut refresh_signal = use_signal(|| 0u64);
    let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());
    let mut toast_counter = use_signal(|| 0u64);

    // Mount: probe session. 401 → nav back to /helper with Toast.
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::get_helper_session(&config).await {
                Ok(s) => {
                    session.set(Some(s));
                }
                Err(_) => {
                    nav.replace(Route::HelperLogin {});
                    return;
                }
            }
            loading.set(false);
        });
    });

    let on_logout = move |_| {
        spawn(async move {
            let config = CONFIG.read().clone();
            // Best effort: even if logout fails, navigate back to /helper.
            // Backend will clear stale sessions on next cookie check at latest.
            let _ = api::helper_logout(&config).await;
            nav.replace(Route::HelperLogin {});
        });
    };

    let session_data = session.read().clone();
    let assembly_name = session_data.as_ref().map(|s| s.assembly_name.clone());
    let assembly_id_opt = session_data.as_ref().map(|s| s.assembly_id);

    rsx! {
        HelperShell {
            assembly_name: assembly_name,
            on_logout: on_logout,
            if *loading.read() {
                div { class: "p-4", "{i18n.t(Key::Loading)}" }
            } else if let Some(assembly_id) = assembly_id_opt {
                ConnectionBanner { visible: *conn_lost.read() }
                LiveCounter {
                    assembly_id: assembly_id,
                    polling_enabled: true,
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
                    read_only: false,
                    refresh_signal: refresh_signal,
                    // Smart parent wires the toggle: call API, then bump refresh_signal on success.
                    // SAME wiring as AttendanceTab in assembly_details.rs (ATTN-06).
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
                                Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
                            }
                        });
                    },
                }
            }
        }
        ToastContainer { messages: toast_messages }
    }
}
