//! AssemblyStatusBadge (Phase 4 Plan 06) — wiederverwendbar in Liste + Detail-Header.
//!
//! Pattern direkt aus application_list.rs:7-27 (status_label / status_badge_class) —
//! als eigene Component verpackt, weil mehrfach reused (D-13).
use dioxus::prelude::*;

use crate::api::AssemblyStatusTO;
use crate::i18n::{use_i18n, Key};

fn status_label(i18n: &crate::i18n::I18n, status: &AssemblyStatusTO) -> String {
    match status {
        AssemblyStatusTO::Preparation => i18n.t(Key::AssemblyStatusPreparation).to_string(),
        AssemblyStatusTO::Open => i18n.t(Key::AssemblyStatusOpen).to_string(),
        AssemblyStatusTO::Closed => i18n.t(Key::AssemblyStatusClosed).to_string(),
    }
}

fn status_badge_class(status: &AssemblyStatusTO) -> &'static str {
    // UI-SPEC §Status-Badge palette: gray (Preparation), green (Open), blue (Closed)
    match status {
        AssemblyStatusTO::Preparation => {
            "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium"
        }
        AssemblyStatusTO::Open => {
            "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
        }
        AssemblyStatusTO::Closed => {
            "bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs font-medium"
        }
    }
}

#[component]
pub fn AssemblyStatusBadge(status: AssemblyStatusTO) -> Element {
    let i18n = use_i18n();
    let label = status_label(&i18n, &status);
    let class = status_badge_class(&status);
    rsx! { span { class: "{class}", "{label}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_class_preparation_is_gray() {
        let class = status_badge_class(&AssemblyStatusTO::Preparation);
        assert!(class.contains("bg-gray-100"));
        assert!(class.contains("text-gray-800"));
    }

    #[test]
    fn badge_class_open_is_green() {
        let class = status_badge_class(&AssemblyStatusTO::Open);
        assert!(class.contains("bg-green-100"));
        assert!(class.contains("text-green-800"));
    }

    #[test]
    fn badge_class_closed_is_blue() {
        let class = status_badge_class(&AssemblyStatusTO::Closed);
        assert!(class.contains("bg-blue-100"));
        assert!(class.contains("text-blue-800"));
    }

    #[test]
    fn badge_class_uses_consistent_pill_classes() {
        for status in &[
            AssemblyStatusTO::Preparation,
            AssemblyStatusTO::Open,
            AssemblyStatusTO::Closed,
        ] {
            let class = status_badge_class(status);
            assert!(class.contains("px-2"));
            assert!(class.contains("py-1"));
            assert!(class.contains("rounded"));
            assert!(class.contains("text-xs"));
            assert!(class.contains("font-medium"));
        }
    }
}
