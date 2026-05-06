//! QrCard Component (Phase 4 Plan 05) — Print-fähige Helfer-Token-Card.
//!
//! `qr_svg` kommt aus dem `POST /api/assembly/{id}/helper-tokens` Response — das Backend
//! (Phase 2 D-21 / T-04-20) ist der trusted producer des SVG-Markups. Es fließt KEIN
//! User-Input in `qr_svg` ein, daher ist `dangerous_inner_html` hier sicher.
//!
//! Print-Pfad: `window.print()` — CSS `@media print` (Plan 02 in `input.css`) blendet
//! alles außer der `.qr-card` aus. Single-Card-Print pro Klick (Bulk-Print ist v2).
//!
//! Pure-Logic-Helper `format_card_title` ist Cargo-testbar — verifiziert dass der
//! sichtbare Titel korrekt das Memo eingebettet bekommt.

use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key, I18n};

/// Pure helper: baut den Card-Titel aus i18n-Prefix + Memo. Cargo-testbar (kein web-sys).
#[allow(dead_code)]
pub fn format_card_title(i18n: &I18n, memo: &str) -> String {
    format!("{} {}", i18n.t(Key::HelperTokenCardTitle), memo)
}

/// ADR-2026-05-06: pure helper that builds the magic-login-link from a
/// browser origin and a plain-text code. Cargo-testbar (no web-sys).
///
/// Format mirrors the backend QR-payload from `HelperTokenServiceImpl::
/// create_helper_token` (Phase 2 Plan 02-05): `{origin}/helper?code={code}`.
/// Trailing slashes on the origin are normalised — the user-facing URL must
/// not contain a doubled slash like `https://example.com//helper`.
#[allow(dead_code)]
pub fn format_magic_link(origin: &str, code: &str) -> String {
    format!("{}/helper?code={}", origin.trim_end_matches('/'), code)
}

/// ADR-2026-05-06: read the page origin via `window.location.origin()` at
/// render time. Returns an empty string if the call fails — the caller
/// suppresses the magic-link section in that case.
fn read_window_origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

#[component]
pub fn QrCard(memo: String, code: String, qr_svg: String) -> Element {
    let i18n = use_i18n();
    let title = format_card_title(&i18n, &memo);
    let origin = read_window_origin();
    let magic_link = if origin.is_empty() {
        String::new()
    } else {
        format_magic_link(&origin, &code)
    };
    let mut copied = use_signal(|| false);
    let link_for_copy = magic_link.clone();
    rsx! {
        div { class: "qr-card bg-white border border-gray-300 rounded-lg p-6 shadow-sm flex flex-col items-center gap-4 max-w-sm mx-auto",
            h2 { class: "text-lg font-semibold text-gray-800", "{title}" }
            p { class: "text-xs text-amber-700 print:hidden text-center max-w-xs",
                "{i18n.t(Key::HelperTokenWarning)}"
            }
            // Trusted producer: Backend D-21 SVG (kein User-Input). XSS-Risiko ausgeschlossen.
            div { class: "w-64 h-64", dangerous_inner_html: "{qr_svg}" }
            p { class: "text-sm text-gray-600 text-center",
                "{i18n.t(Key::HelperTokenCardManualHint)}"
            }
            p { class: "font-mono text-2xl font-semibold tracking-widest text-gray-900 select-all",
                "{code}"
            }
            // ADR-2026-05-06: Magic-Link section — visible only when the
            // browser origin could be derived. Pre-printed cards do NOT
            // include this (print:hidden) — the QR-Code already encodes
            // the same URL.
            if !magic_link.is_empty() {
                div { class: "flex flex-col items-center gap-2 print:hidden w-full",
                    span { class: "text-sm font-medium text-gray-700",
                        "{i18n.t(Key::HelperTokenLoginLink)}"
                    }
                    a {
                        class: "text-sm text-blue-600 hover:text-blue-800 underline break-all text-center",
                        href: "{magic_link}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "{magic_link}"
                    }
                    button {
                        class: "text-sm bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-1 rounded min-h-[36px]",
                        onclick: move |_| {
                            let to_copy = link_for_copy.clone();
                            let mut copied_signal = copied;
                            spawn(async move {
                                if let Some(window) = web_sys::window() {
                                    let clipboard = window.navigator().clipboard();
                                    let promise = clipboard.write_text(&to_copy);
                                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                                }
                                copied_signal.set(true);
                            });
                        },
                        if *copied.read() {
                            "{i18n.t(Key::Copied)}"
                        } else {
                            "{i18n.t(Key::Copy)}"
                        }
                    }
                }
            }
            button {
                class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded print:hidden min-h-[44px]",
                onclick: move |_| {
                    if let Some(window) = web_sys::window() {
                        let _ = window.print();
                    }
                },
                "{i18n.t(Key::HelperTokenPrint)}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;

    #[test]
    fn title_includes_memo_de() {
        let i18n = I18n::new(Locale::De);
        let title = format_card_title(&i18n, "Anna");
        assert!(
            title.contains("Anna"),
            "title must embed the memo, got: {title}"
        );
    }

    #[test]
    fn title_includes_memo_en() {
        let i18n = I18n::new(Locale::En);
        let title = format_card_title(&i18n, "Bernd");
        assert!(title.contains("Bernd"), "title must embed memo (en locale)");
    }

    #[test]
    fn title_handles_empty_memo() {
        let i18n = I18n::new(Locale::De);
        let title = format_card_title(&i18n, "");
        // Should produce "<prefix> " — non-empty even with empty memo.
        assert!(!title.is_empty());
    }

    #[test]
    fn magic_link_uses_helper_path_and_code_param() {
        let link = format_magic_link("https://example.com", "ABC1234567");
        assert_eq!(link, "https://example.com/helper?code=ABC1234567");
    }

    #[test]
    fn magic_link_normalises_trailing_slash() {
        // ADR-2026-05-06: doubled slashes break some path-strict routers.
        let link = format_magic_link("https://example.com/", "ABC1234567");
        assert_eq!(
            link, "https://example.com/helper?code=ABC1234567",
            "trailing slash on origin must be stripped before joining"
        );
    }

    #[test]
    fn magic_link_handles_localhost_origin() {
        let link = format_magic_link("http://localhost:8080", "Z9X8C7V6B5");
        assert_eq!(link, "http://localhost:8080/helper?code=Z9X8C7V6B5");
    }
}
