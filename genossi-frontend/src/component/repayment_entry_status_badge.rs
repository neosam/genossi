//! Phase 12 — RepaymentEntryStatusBadge (Offen/Angeschrieben/Ausbezahlt).
//!
//! 1:1-Klon von `assembly_status_badge.rs` mit angepasstem Status-Enum
//! (`RepaymentEntryStatusTO` aus Plan 12-01) und Farbpalette laut
//! CONTEXT D-14: Offen=grau, Angeschrieben=blau, Ausbezahlt=grün.
use dioxus::prelude::*;

use crate::api::RepaymentEntryStatusTO;
use crate::i18n::{use_i18n, Key};

fn status_label(i18n: &crate::i18n::I18n, status: &RepaymentEntryStatusTO) -> String {
    match status {
        RepaymentEntryStatusTO::Open => i18n.t(Key::RepaymentEntryStatusOpen).to_string(),
        RepaymentEntryStatusTO::Contacted => {
            i18n.t(Key::RepaymentEntryStatusContacted).to_string()
        }
        RepaymentEntryStatusTO::PaidOut => i18n.t(Key::RepaymentEntryStatusPaidOut).to_string(),
    }
}

fn status_badge_class(status: &RepaymentEntryStatusTO) -> &'static str {
    // CONTEXT D-14: Offen=grau, Angeschrieben=blau, Ausbezahlt=grün
    match status {
        RepaymentEntryStatusTO::Open => {
            "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium"
        }
        RepaymentEntryStatusTO::Contacted => {
            "bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs font-medium"
        }
        RepaymentEntryStatusTO::PaidOut => {
            "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
        }
    }
}

#[component]
pub fn RepaymentEntryStatusBadge(status: RepaymentEntryStatusTO) -> Element {
    let i18n = use_i18n();
    let label = status_label(&i18n, &status);
    let class = status_badge_class(&status);
    rsx! { span { class: "{class}", "{label}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_open_is_gray() {
        let class = status_badge_class(&RepaymentEntryStatusTO::Open);
        assert!(class.contains("bg-gray-100"));
        assert!(class.contains("text-gray-800"));
    }

    #[test]
    fn class_contacted_is_blue() {
        let class = status_badge_class(&RepaymentEntryStatusTO::Contacted);
        assert!(class.contains("bg-blue-100"));
        assert!(class.contains("text-blue-800"));
    }

    #[test]
    fn class_paidout_is_green() {
        let class = status_badge_class(&RepaymentEntryStatusTO::PaidOut);
        assert!(class.contains("bg-green-100"));
        assert!(class.contains("text-green-800"));
    }

    #[test]
    fn all_share_pill_styling() {
        for s in &[
            RepaymentEntryStatusTO::Open,
            RepaymentEntryStatusTO::Contacted,
            RepaymentEntryStatusTO::PaidOut,
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
