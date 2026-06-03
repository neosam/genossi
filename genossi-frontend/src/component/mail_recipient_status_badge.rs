//! Quick 260603-evf — MailRecipientStatusBadge: visual marker for bulk-mail
//! recipient outcomes. Special-cases `failed`+`no_repayment_letter` as amber
//! (distinct from generic `failed`=red) so the Vorstand can spot recoverable
//! failures at a glance and trigger the "Brief generieren + Retry" action.
//!
//! Structural sibling of `repayment_entry_status_badge.rs` — same Tailwind
//! pill-styling convention (`bg-{color}-100 text-{color}-800 px-2 py-1 rounded
//! text-xs font-medium`).
use dioxus::prelude::*;

use crate::i18n::{use_i18n, Key};

/// Quick 260603-evf: re-used by the recipients table and the action-column
/// in `mail_page.rs` to decide whether to render the recovery action.
/// `.starts_with("no_repayment_letter")` keeps the check future-safe in case
/// the worker ever appends contextual detail after a colon.
pub fn is_no_repayment_letter_failure(status: &str, error: Option<&str>) -> bool {
    status == "failed"
        && error
            .map(|e| e.starts_with("no_repayment_letter"))
            .unwrap_or(false)
}

fn status_label_key(status: &str, error: Option<&str>) -> Key {
    if is_no_repayment_letter_failure(status, error) {
        return Key::MailFailedNoRepaymentLetter;
    }
    match status {
        "sent" => Key::MailSent,
        "failed" => Key::MailFailed,
        _ => Key::MailJobPending,
    }
}

fn status_badge_class(status: &str, error: Option<&str>) -> &'static str {
    if is_no_repayment_letter_failure(status, error) {
        return "bg-amber-100 text-amber-800 px-2 py-1 rounded text-xs font-medium";
    }
    match status {
        "sent" => "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium",
        "failed" => "bg-red-100 text-red-800 px-2 py-1 rounded text-xs font-medium",
        _ => "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium",
    }
}

#[component]
pub fn MailRecipientStatusBadge(status: String, error: Option<String>) -> Element {
    let i18n = use_i18n();
    let class = status_badge_class(&status, error.as_deref());
    let label = i18n.t(status_label_key(&status, error.as_deref()));
    rsx! { span { class: "{class}", "{label}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_sent_is_green() {
        let class = status_badge_class("sent", None);
        assert!(class.contains("bg-green-100"));
        assert!(class.contains("text-green-800"));
    }

    #[test]
    fn class_failed_without_error_is_red() {
        let class = status_badge_class("failed", None);
        assert!(class.contains("bg-red-100"));
        assert!(class.contains("text-red-800"));
    }

    #[test]
    fn class_failed_with_no_repayment_letter_is_amber() {
        let class = status_badge_class("failed", Some("no_repayment_letter"));
        assert!(class.contains("bg-amber-100"));
        assert!(class.contains("text-amber-800"));
    }

    #[test]
    fn class_failed_with_no_repayment_letter_detail_suffix_still_amber() {
        // Future-safety: if the worker ever appends ":<details>" we must keep
        // detecting the no_repayment_letter case via `.starts_with`.
        let class = status_badge_class("failed", Some("no_repayment_letter: details"));
        assert!(class.contains("bg-amber-100"));
        assert!(class.contains("text-amber-800"));
    }

    #[test]
    fn class_failed_with_other_error_stays_red() {
        let class = status_badge_class("failed", Some("smtp_timeout"));
        assert!(class.contains("bg-red-100"));
        assert!(class.contains("text-red-800"));
    }

    #[test]
    fn class_pending_and_queued_are_gray() {
        for s in &["pending", "queued"] {
            let class = status_badge_class(s, None);
            assert!(class.contains("bg-gray-100"), "{s} should be gray");
            assert!(class.contains("text-gray-800"), "{s} should be gray");
        }
    }

    #[test]
    fn label_key_matches_status_and_error() {
        assert_eq!(
            status_label_key("failed", Some("no_repayment_letter")),
            Key::MailFailedNoRepaymentLetter,
        );
        assert_eq!(status_label_key("failed", None), Key::MailFailed);
        assert_eq!(status_label_key("sent", None), Key::MailSent);
        assert_eq!(status_label_key("pending", None), Key::MailJobPending);
        // Unknown status falls back to pending label.
        assert_eq!(status_label_key("queued", None), Key::MailJobPending);
    }

    #[test]
    fn is_no_repayment_letter_failure_only_for_failed_status() {
        assert!(is_no_repayment_letter_failure(
            "failed",
            Some("no_repayment_letter"),
        ));
        assert!(!is_no_repayment_letter_failure("failed", None));
        // status must be failed even if the error string matches — guards
        // against future edge cases where the worker sets error on non-failed
        // statuses.
        assert!(!is_no_repayment_letter_failure(
            "sent",
            Some("no_repayment_letter"),
        ));
        assert!(!is_no_repayment_letter_failure(
            "pending",
            Some("no_repayment_letter"),
        ));
    }

    #[test]
    fn all_share_pill_styling() {
        let cases: [(&str, Option<&str>); 4] = [
            ("sent", None),
            ("failed", None),
            ("failed", Some("no_repayment_letter")),
            ("pending", None),
        ];
        for (s, e) in cases {
            let c = status_badge_class(s, e);
            assert!(c.contains("px-2"), "missing px-2 in {c}");
            assert!(c.contains("py-1"), "missing py-1 in {c}");
            assert!(c.contains("rounded"), "missing rounded in {c}");
            assert!(c.contains("text-xs"), "missing text-xs in {c}");
            assert!(c.contains("font-medium"), "missing font-medium in {c}");
        }
    }
}
