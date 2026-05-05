//! HelperLogin Page (Phase 4 Plan 09) — /helper route, public.
//!
//! HLPR-03: zeigt QR-Code-Scan-Button UND ManualCodeInput parallel.
//! D-03: Camera-Permission erst nach Klick.
//! D-06: Auto-Redirect bei vorhandener Session.
//! Status-Code-Mapping für Redeem-Errors (UI-SPEC §"Error state — Redeem").

use dioxus::prelude::*;

use crate::api::{self, AppError};
use crate::component::{HelperShell, ManualCodeInput, QrScanner};
use crate::i18n::{use_i18n, I18n, Key};
use crate::router::Route;
use crate::service::config::CONFIG;

fn map_redeem_error(i18n: &I18n, err: &AppError) -> String {
    match err.status {
        Some(400) => i18n.t(Key::HelperLoginInvalidFormat).to_string(),
        Some(403) => i18n.t(Key::HelperLoginErrorAssemblyClosed).to_string(),
        Some(404) => i18n.t(Key::HelperLoginErrorNotFound).to_string(),
        Some(410) => i18n.t(Key::HelperLoginErrorAlreadyUsed).to_string(),
        Some(429) => i18n.t(Key::HelperLoginErrorRateLimit).to_string(),
        _ => err.message.clone(),
    }
}

// W-05: Delayed loading skeleton — zeigt für die ersten 200ms NICHTS,
// erst danach einen zentrierten Skeleton-Box. Vermeidet Flash-of-Loading
// bei schnellen /api/helper/session-Probes (RESEARCH §UI-SPEC Loading-Pattern).
#[component]
fn DelayedLoadingSkeleton() -> Element {
    let mut visible = use_signal(|| false);
    use_effect(move || {
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(200).await;
            visible.set(true);
        });
    });
    if !*visible.read() {
        return rsx! { div { class: "h-1" } }; // 1px placeholder, no flash
    }
    rsx! {
        div { class: "flex items-center justify-center py-12",
            div { class: "animate-pulse flex flex-col gap-2 w-64",
                div { class: "h-4 bg-gray-200 rounded w-3/4 mx-auto" }
                div { class: "h-4 bg-gray-200 rounded w-1/2 mx-auto" }
            }
        }
    }
}

/// Spawns the redeem-async-task. All inputs are Copy (Signals + Navigator) or
/// Clone-by-value (I18n) so this helper can be called from multiple submit
/// closures without move conflicts.
fn spawn_redeem(
    code: String,
    i18n: I18n,
    nav: dioxus_router::prelude::Navigator,
    mut submitting: Signal<bool>,
    mut error_msg: Signal<Option<String>>,
) {
    spawn(async move {
        submitting.set(true);
        error_msg.set(None);
        let config = CONFIG.read().clone();
        match api::redeem_helper_token(&config, code).await {
            Ok(_) => {
                nav.replace(Route::HelperAttendance {});
            }
            Err(e) => {
                error_msg.set(Some(map_redeem_error(&i18n, &e)));
            }
        }
        submitting.set(false);
    });
}

#[component]
pub fn HelperLogin() -> Element {
    let i18n = use_i18n();
    let nav = navigator();

    let mut show_scanner = use_signal(|| false);
    let submitting = use_signal(|| false);
    let error_msg = use_signal(|| Option::<String>::None);
    let mut redirect_check_done = use_signal(|| false);

    // Auto-Redirect (D-06): probe /api/helper/session on mount.
    // 200 → already authenticated → nav.replace zu /helper/attendance.
    // 401 → render Login-UI (set redirect_check_done=true).
    use_effect(move || {
        spawn(async move {
            let config = CONFIG.read().clone();
            if api::get_helper_session(&config).await.is_ok() {
                nav.replace(Route::HelperAttendance {});
                return;
            }
            redirect_check_done.set(true);
        });
    });

    // Clone i18n once per submit-closure so each branch has its own owned value.
    let i18n_manual = i18n.clone();
    let i18n_qr = i18n.clone();
    let mut error_msg_for_qr_err = error_msg;

    rsx! {
        HelperShell {
            assembly_name: None,
            on_logout: move |_| {}, // logout meaningless on login page; no-op
            if !*redirect_check_done.read() {
                // W-05: kein Spinner unter ~200ms (delayed via gloo_timers).
                DelayedLoadingSkeleton {}
            } else {
                div { class: "py-8",
                    h1 { class: "text-2xl font-bold mb-2", "{i18n.t(Key::HelperLoginTitle)}" }
                    p { class: "text-gray-600 mb-8", "{i18n.t(Key::HelperLoginSubtitle)}" }

                    div { class: "flex flex-col md:flex-row gap-6 md:gap-12 items-stretch",
                        // Left path: QR-Code-Scan
                        div { class: "flex-1 flex flex-col gap-3",
                            button {
                                class: "bg-blue-600 hover:bg-blue-700 text-white font-medium px-6 py-4 rounded-lg w-full text-lg min-h-[44px]",
                                onclick: move |_| show_scanner.set(true),
                                span { class: "mr-2", "\u{1F4F7}" } // camera glyph
                                "{i18n.t(Key::HelperLoginScanQR)}"
                            }
                        }
                        // Divider (visible on desktop)
                        div { class: "text-center text-gray-400 text-sm md:flex md:items-center md:px-4",
                            "oder"
                        }
                        // Right path: Manual-Code
                        div { class: "flex-1",
                            ManualCodeInput {
                                on_submit: move |code: String| {
                                    spawn_redeem(code, i18n_manual.clone(), nav, submitting, error_msg);
                                },
                                submitting: *submitting.read(),
                                error: error_msg.read().clone(),
                            }
                        }
                    }

                    if *show_scanner.read() {
                        QrScanner {
                            on_scan: move |code: String| {
                                show_scanner.set(false);
                                spawn_redeem(code, i18n_qr.clone(), nav, submitting, error_msg);
                            },
                            on_error: move |msg: String| {
                                show_scanner.set(false);
                                error_msg_for_qr_err.set(Some(msg));
                            },
                            on_cancel: move |_| show_scanner.set(false),
                        }
                    }
                }
            }
        }
    }
}
