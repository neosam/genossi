//! ManualCodeInput Component (Phase 4 Plan 05) — HLPR-03 Manual-Code-Fallback.
//!
//! Live-filters input to 10-char Crockford Base32 (UX-only — backend D-24 authoritative).
//! Used in `helper_login.rs` parallel to `QrScanner` — both paths route to
//! `api::redeem_helper_token`.
//!
//! Trust boundary: This component is UX-only. Frontend-Validation (Submit-Button-Disable,
//! Crockford-Filter) ist KEIN Security-Boundary — Backend D-24 (Phase 2 Plan 02-05) bleibt
//! authoritative. Ein User mit DevTools kann die Disable-Logik aushebeln; Backend lehnt
//! invalid codes mit 400 ab (T-04-21 Mitigation).
//!
//! Pure-Logic-Tests (`is_valid_helper_code`, `sanitize_helper_code_input`) leben in
//! `helper_code.rs` und sind aus Plan 02 grün. Plan 05 ergänzt einen Cargo-Test, der
//! verifiziert dass invalid Crockford-Strings den Submit blocken (`compute_submit_state`).

use dioxus::prelude::*;

use crate::helper_code::{is_valid_helper_code, sanitize_helper_code_input};
use crate::i18n::{use_i18n, Key};

/// Pure-logic helper: berechnet den Submit-Disabled-State + ob die Eingabe valid ist.
/// Cargo-testbar ohne web-sys / Dioxus-Render-Cycle.
///
/// Returns `(valid, submit_disabled)`:
/// - `valid` — exakte 10 Chars aus Crockford-Alphabet
/// - `submit_disabled` — wenn entweder invalid ODER `submitting`
#[allow(dead_code)]
pub fn compute_submit_state(value: &str, submitting: bool) -> (bool, bool) {
    let valid = is_valid_helper_code(value);
    (valid, !valid || submitting)
}

#[component]
pub fn ManualCodeInput(
    on_submit: EventHandler<String>,
    submitting: bool,
    error: Option<String>,
) -> Element {
    let i18n = use_i18n();
    let mut value = use_signal(String::new);

    let (_, submit_disabled) = compute_submit_state(&value.read(), submitting);

    rsx! {
        form {
            class: "flex flex-col gap-3",
            onsubmit: move |e| {
                e.prevent_default();
                if !submit_disabled {
                    on_submit.call((*value.read()).clone());
                }
            },
            label { class: "text-sm font-medium text-gray-700",
                "{i18n.t(Key::HelperLoginManualHeading)}"
            }
            input {
                class: "font-mono text-2xl tracking-widest text-center uppercase w-full px-4 py-3 border-2 border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:bg-gray-100",
                r#type: "text",
                maxlength: "10",
                autocapitalize: "characters",
                inputmode: "text",
                autocomplete: "off",
                spellcheck: "false",
                placeholder: "{i18n.t(Key::HelperLoginManualPlaceholder)}",
                value: "{value}",
                disabled: submitting,
                oninput: move |e| {
                    let cleaned = sanitize_helper_code_input(&e.value());
                    value.set(cleaned);
                },
            }
            if let Some(msg) = error.as_ref() {
                p { class: "text-sm text-red-600 -mt-1", "{msg}" }
            }
            button {
                r#type: "submit",
                class: "bg-blue-600 hover:bg-blue-700 text-white font-medium px-6 py-3 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2 min-h-[44px]",
                disabled: submit_disabled,
                if submitting {
                    span { class: "animate-spin inline-block h-4 w-4 border-2 border-white border-t-transparent rounded-full" }
                }
                span { "{i18n.t(Key::HelperLoginSubmit)}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_short_code_blocks_submit() {
        let (valid, disabled) = compute_submit_state("ABC", false);
        assert!(!valid, "3-char code is not valid");
        assert!(disabled, "submit must be disabled when invalid");
    }

    #[test]
    fn invalid_chars_block_submit() {
        // Contains 'I' (excluded from Crockford) — must stay disabled.
        let (valid, disabled) = compute_submit_state("ABCIDEFGHJ", false);
        assert!(!valid, "code containing 'I' is not valid Crockford");
        assert!(disabled, "submit must be disabled for excluded chars");
    }

    #[test]
    fn lowercase_blocks_submit() {
        let (valid, disabled) = compute_submit_state("abc1234567", false);
        assert!(!valid, "lowercase is not valid (sanitize would uppercase first)");
        assert!(disabled);
    }

    #[test]
    fn empty_value_blocks_submit() {
        let (valid, disabled) = compute_submit_state("", false);
        assert!(!valid);
        assert!(disabled);
    }

    #[test]
    fn valid_code_enables_submit_when_idle() {
        let (valid, disabled) = compute_submit_state("ABC1234567", false);
        assert!(valid, "10-char Crockford uppercase is valid");
        assert!(!disabled, "valid + not submitting → submit enabled");
    }

    #[test]
    fn valid_code_blocks_submit_while_submitting() {
        let (valid, disabled) = compute_submit_state("ABC1234567", true);
        assert!(valid, "value itself is still valid Crockford");
        assert!(disabled, "submitting=true forces disabled (no double-submit)");
    }

    #[test]
    fn sanitize_filters_invalid_chars_to_valid_state() {
        // Simulate full input pipeline: user types junk, sanitize cleans, then validate.
        let cleaned = sanitize_helper_code_input("abc123!@#XY9012ZZ");
        let (valid, disabled) = compute_submit_state(&cleaned, false);
        // sanitize uppercases, filters non-Crockford, takes 10 chars.
        // "abc123!@#XY9012ZZ" → upper "ABC123!@#XY9012ZZ" → keep "ABC123XY9012ZZ" → take 10 → "ABC123XY90"
        assert_eq!(cleaned, "ABC123XY90");
        assert!(valid, "sanitized to exactly 10 valid chars");
        assert!(!disabled);
    }
}
