//! Phase 12 — RepaymentPhaseStatusBadge (Vorbereitung/Offen/Abgeschlossen).
//!
//! 1:1-Klon von `assembly_status_badge.rs` mit angepasstem Status-Enum
//! (`RepaymentPhaseStatusTO` aus Plan 12-01) und Farbpalette laut
//! CONTEXT D-14 + Claude's Discretion: Vorbereitung=grau, Offen=blau,
//! Abgeschlossen=grün. (Assembly-Badge nutzt Open=grün, Closed=blau —
//! Phase 12 dreht diese Farben für RepaymentPhase.)
use dioxus::prelude::*;

use crate::api::RepaymentPhaseStatusTO;
use crate::i18n::{use_i18n, Key};

fn status_label(i18n: &crate::i18n::I18n, status: &RepaymentPhaseStatusTO) -> String {
    match status {
        RepaymentPhaseStatusTO::Preparation => {
            i18n.t(Key::RepaymentPhaseStatusPreparation).to_string()
        }
        RepaymentPhaseStatusTO::Open => i18n.t(Key::RepaymentPhaseStatusOpen).to_string(),
        RepaymentPhaseStatusTO::Closed => i18n.t(Key::RepaymentPhaseStatusClosed).to_string(),
    }
}

fn status_badge_class(status: &RepaymentPhaseStatusTO) -> &'static str {
    // CONTEXT D-14 + Claude's Discretion: Vorbereitung=grau, Offen=blau, Abgeschlossen=grün
    match status {
        RepaymentPhaseStatusTO::Preparation => {
            "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium"
        }
        RepaymentPhaseStatusTO::Open => {
            "bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs font-medium"
        }
        RepaymentPhaseStatusTO::Closed => {
            "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
        }
    }
}

#[component]
pub fn RepaymentPhaseStatusBadge(status: RepaymentPhaseStatusTO) -> Element {
    let i18n = use_i18n();
    let label = status_label(&i18n, &status);
    let class = status_badge_class(&status);
    rsx! { span { class: "{class}", "{label}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_preparation_is_gray() {
        let class = status_badge_class(&RepaymentPhaseStatusTO::Preparation);
        assert!(class.contains("bg-gray-100"));
        assert!(class.contains("text-gray-800"));
    }

    #[test]
    fn class_open_is_blue() {
        let class = status_badge_class(&RepaymentPhaseStatusTO::Open);
        assert!(class.contains("bg-blue-100"));
        assert!(class.contains("text-blue-800"));
    }

    #[test]
    fn class_closed_is_green() {
        let class = status_badge_class(&RepaymentPhaseStatusTO::Closed);
        assert!(class.contains("bg-green-100"));
        assert!(class.contains("text-green-800"));
    }

    #[test]
    fn all_share_pill_styling() {
        for s in &[
            RepaymentPhaseStatusTO::Preparation,
            RepaymentPhaseStatusTO::Open,
            RepaymentPhaseStatusTO::Closed,
        ] {
            let c = status_badge_class(s);
            assert!(c.contains("px-2"), "missing px-2 in {c}");
            assert!(c.contains("py-1"), "missing py-1 in {c}");
            assert!(c.contains("rounded"), "missing rounded in {c}");
            assert!(c.contains("text-xs"), "missing text-xs in {c}");
            assert!(c.contains("font-medium"), "missing font-medium in {c}");
        }
    }
}
