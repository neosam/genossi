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

/// ADR-2026-05-06: pure helper that extracts the `code` query parameter
/// from a URL search string (e.g. `?code=ABC1234567` or `?foo=bar&code=…`).
/// Returns `None` if the parameter is absent or empty. Cargo-testbar (no
/// web-sys).
///
/// We deliberately implement a minimal parser instead of pulling in `url`
/// or `urlencoding` — the magic-link only ever sends a 10-char Crockford-
/// Base32 string, which has no characters that require percent-decoding.
#[allow(dead_code)]
pub fn extract_code_query_param(search: &str) -> Option<String> {
    let trimmed = search.trim_start_matches('?');
    if trimmed.is_empty() {
        return None;
    }
    for pair in trimmed.split('&') {
        let mut split = pair.splitn(2, '=');
        let key = split.next().unwrap_or("");
        let value = split.next().unwrap_or("");
        if key == "code" && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

/// Read the current URL's query string via `window.location.search()`.
/// Returns an empty string if the call fails (test/SSR contexts).
fn read_window_search() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
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
    // ADR-2026-05-06: ?code= magic-link state. `initial_code` pre-fills the
    // ManualCodeInput when the redeem fails (so the user can retry).
    let initial_code = use_signal(|| extract_code_query_param(&read_window_search()));

    // Auto-Redirect (D-06): probe /api/helper/session on mount.
    // 200 → already authenticated → nav.replace zu /helper/attendance.
    // 401 → render Login-UI (set redirect_check_done=true). On 401 we ALSO
    // auto-submit the redeem if the magic-link supplied a `?code=` query
    // parameter (ADR-2026-05-06). Edge case: if the redeem fails (already
    // used, revoked, etc.) the existing map_redeem_error path renders the
    // inline error and the pre-filled code stays visible for manual retry.
    let i18n_for_effect = i18n.clone();
    use_effect(move || {
        let i18n = i18n_for_effect.clone();
        spawn(async move {
            let config = CONFIG.read().clone();
            if api::get_helper_session(&config).await.is_ok() {
                nav.replace(Route::HelperAttendance {});
                return;
            }
            redirect_check_done.set(true);
            // After confirming there is no existing helper session, attempt
            // the magic-link auto-submit. spawn_redeem already handles the
            // navigation on success and the error path on failure.
            if let Some(code) = (*initial_code.read()).clone() {
                spawn_redeem(code, i18n, nav, submitting, error_msg);
            }
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
                        // Right path: Manual-Code (pre-filled by ?code= magic-link).
                        div { class: "flex-1",
                            ManualCodeInput {
                                on_submit: move |code: String| {
                                    spawn_redeem(code, i18n_manual.clone(), nav, submitting, error_msg);
                                },
                                submitting: *submitting.read(),
                                error: error_msg.read().clone(),
                                initial_value: (*initial_code.read()).clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_returns_none_for_empty_search() {
        assert_eq!(extract_code_query_param(""), None);
        assert_eq!(extract_code_query_param("?"), None);
    }

    #[test]
    fn extract_code_returns_value_when_only_param() {
        assert_eq!(
            extract_code_query_param("?code=ABC1234567"),
            Some("ABC1234567".to_string())
        );
    }

    #[test]
    fn extract_code_returns_value_without_leading_question_mark() {
        // window.location.search() in modern browsers includes the '?', but
        // be defensive — some embedders strip it.
        assert_eq!(
            extract_code_query_param("code=ABC1234567"),
            Some("ABC1234567".to_string())
        );
    }

    #[test]
    fn extract_code_skips_other_params() {
        assert_eq!(
            extract_code_query_param("?foo=bar&code=Z9X8C7V6B5&baz=qux"),
            Some("Z9X8C7V6B5".to_string())
        );
    }

    #[test]
    fn extract_code_returns_none_when_param_absent() {
        assert_eq!(extract_code_query_param("?foo=bar"), None);
    }

    #[test]
    fn extract_code_returns_none_when_value_empty() {
        // Defensive: ?code= with empty value must NOT trigger auto-submit.
        // The frontend is the last line of defence before a backend 400.
        assert_eq!(extract_code_query_param("?code="), None);
    }

    #[test]
    fn extract_code_handles_code_as_first_of_many() {
        assert_eq!(
            extract_code_query_param("?code=ABC1234567&extra=1"),
            Some("ABC1234567".to_string())
        );
    }
}
