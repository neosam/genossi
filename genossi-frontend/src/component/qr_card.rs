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

#[component]
pub fn QrCard(memo: String, code: String, qr_svg: String) -> Element {
    let i18n = use_i18n();
    let title = format_card_title(&i18n, &memo);
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
}
