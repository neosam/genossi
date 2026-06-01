//! Repayment phase detail page (Phase 12 Plan 12-05, UI-02) — admin-only.
//!
//! Task 1: pure status-driven render-logic predicates (should_show_open_button,
//! should_show_close_button, is_share_value_editable). Plan 12-06 reuses
//! is_share_value_editable for the inline-edit guard.
//!
//! Task 2: 3-Tab-Layout (D-06) with TabStrip + inline BasicsTab + Schließen-
//! Confirm-Modal + 409 CloseConflictResponse body-parse.
//!
//! Task 2 (TDD RED): parse_close_conflict is stubbed (returns None for all).
//! Task 2 GREEN replaces with real serde_json::from_str dispatch.

use crate::api::{AppError, CloseConflictResponse, RepaymentPhaseStatusTO};

/// D-03 + D-08: Öffnen-Button is only visible in status `Preparation`.
fn should_show_open_button(status: RepaymentPhaseStatusTO) -> bool {
    matches!(status, RepaymentPhaseStatusTO::Preparation)
}

/// D-03 + D-08: Schließen-Button is only visible in status `Open`.
fn should_show_close_button(status: RepaymentPhaseStatusTO) -> bool {
    matches!(status, RepaymentPhaseStatusTO::Open)
}

/// D-05 + D-08: `share_value` is read-only in status `Closed`.
/// Plan 12-06 reuses this for the inline-edit guard.
pub(crate) fn is_share_value_editable(status: RepaymentPhaseStatusTO) -> bool {
    !matches!(status, RepaymentPhaseStatusTO::Closed)
}

/// D-04 + Open-Question 5: parses the 409-detail body of POST /api/repayment-phase/{id}/close
/// into a CloseConflictResponse. Returns None when the body is not a valid
/// CloseConflictResponse (e.g. non-409 errors, missing detail body, or garbled JSON).
/// On Some, the caller renders an ErrorAlert with pending_count + member-number list
/// (detail-expand). On None, the caller falls back to a generic Toast with the error message.
fn parse_close_conflict(_err: &AppError) -> Option<CloseConflictResponse> {
    // RED stub: will be replaced in GREEN.
    None
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

    #[test]
    fn parse_close_conflict_returns_none_on_non_409() {
        let err = AppError {
            status: Some(404),
            message: "Not found".into(),
            detail: None,
        };
        assert!(parse_close_conflict(&err).is_none());
    }

    #[test]
    fn parse_close_conflict_returns_none_when_detail_missing() {
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: None,
        };
        assert!(parse_close_conflict(&err).is_none());
    }

    #[test]
    fn parse_close_conflict_returns_some_on_valid_body() {
        let body = r#"{"error":"pending entries","pending_count":2,"pending_member_numbers":["42","43"]}"#;
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: Some(body.to_string()),
        };
        let cc = parse_close_conflict(&err).expect("should parse");
        assert_eq!(cc.pending_count, 2);
        assert_eq!(
            cc.pending_member_numbers,
            vec!["42".to_string(), "43".to_string()]
        );
    }

    #[test]
    fn parse_close_conflict_returns_none_on_garbled_body() {
        let err = AppError {
            status: Some(409),
            message: "Conflict".into(),
            detail: Some("not json".into()),
        };
        assert!(parse_close_conflict(&err).is_none());
    }
}
