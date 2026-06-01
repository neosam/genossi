//! Repayment phase detail page (Phase 12 Plan 12-05, UI-02) — admin-only.
//!
//! Task 1 (TDD RED): pure status-driven render-logic functions exist as stubs,
//! tests are written but FAIL. Task 1 GREEN turns them into real predicates.
//! Task 2 builds the full TabStrip + BasicsTab UI on top.

use crate::api::RepaymentPhaseStatusTO;

/// D-03 + D-08: Öffnen-Button is only visible in status `Preparation`.
fn should_show_open_button(_status: RepaymentPhaseStatusTO) -> bool {
    // RED stub: will be replaced in GREEN.
    false
}

/// D-03 + D-08: Schließen-Button is only visible in status `Open`.
fn should_show_close_button(_status: RepaymentPhaseStatusTO) -> bool {
    // RED stub: will be replaced in GREEN.
    false
}

/// D-05 + D-08: `share_value` is read-only in status `Closed`.
/// Plan 12-06 reuses this for the inline-edit guard.
pub(crate) fn is_share_value_editable(_status: RepaymentPhaseStatusTO) -> bool {
    // RED stub: will be replaced in GREEN.
    false
}

use dioxus::prelude::*;

/// Plan 12-05 Task 2 replaces this stub with the full 3-tab detail page.
#[component]
pub fn RepaymentPhaseDetails(id: String) -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-6",
            h1 { class: "text-2xl font-bold", "Phase-Details" }
            p { class: "text-gray-500 mt-4", "TODO Plan 12-05 Task 2: 3-Tab-Layout (UI-02). Phase-ID: {id}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_button_only_in_preparation() {
        assert!(should_show_open_button(RepaymentPhaseStatusTO::Preparation));
        assert!(!should_show_open_button(RepaymentPhaseStatusTO::Open));
        assert!(!should_show_open_button(RepaymentPhaseStatusTO::Closed));
    }

    #[test]
    fn close_button_only_in_open() {
        assert!(!should_show_close_button(RepaymentPhaseStatusTO::Preparation));
        assert!(should_show_close_button(RepaymentPhaseStatusTO::Open));
        assert!(!should_show_close_button(RepaymentPhaseStatusTO::Closed));
    }

    #[test]
    fn share_value_readonly_in_closed() {
        assert!(is_share_value_editable(RepaymentPhaseStatusTO::Preparation));
        assert!(is_share_value_editable(RepaymentPhaseStatusTO::Open));
        assert!(!is_share_value_editable(RepaymentPhaseStatusTO::Closed));
    }
}
