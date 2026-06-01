//! Inline-Cell-Edit fuer share_count_to_pay_out in der RepaymentEntryList (D-13).
//!
//! Phase-12-Eigen-Design — KEIN Codebase-Analog:
//! - `member_details.rs` nutzt Page-Level-Edit-Toggle (alle Felder gleichzeitig)
//! - Diese Component ist Cell-Level (Click-Zelle -> Input -> Save/Cancel)
//!
//! Spezialisiert auf i32 (Open-Question 3): wenn weitere Inline-Cell-Edit-Cases
//! auftauchen (v1.2+), refactor zu `EditableCell<T>`. Aktuell ein Use-Case.

/// Backend-Constraint (Phase 8 D-11.3 + CHECK-Constraint): share_count_to_pay_out > 0.
pub fn is_share_count_valid(_n: i32) -> bool {
    // RED stub: deliberately wrong so tests fail.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_count_valid_positive() {
        assert!(is_share_count_valid(1));
        assert!(is_share_count_valid(100));
        assert!(is_share_count_valid(i32::MAX));
    }

    #[test]
    fn share_count_invalid_zero() {
        assert!(!is_share_count_valid(0));
    }

    #[test]
    fn share_count_invalid_negative() {
        assert!(!is_share_count_valid(-1));
        assert!(!is_share_count_valid(i32::MIN));
    }
}
