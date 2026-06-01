//! PaidOut-Bulk-Confirm-Modal (Phase 12 Plan 12-10, UI-05).
//!
//! Single-Backend-Endpoint (D-15) -> Sequential-Loop pro Entry.
//! Confirm-Inhalt: Listentabelle + Gesamtsumme + 3-Punkt-Warnung (D-16).
//! Pro Loop-Fehler ein show_toast (D-17); am Ende 1 Summary-Toast (D-15).
//! Nach Loop: refresh_members (Pitfall 3 — current_shares-Cascade).
//!
//! ## Pure Helper
//!
//! `sum_payout_amounts(entries, share_value_cents)` ist die testbare
//! Reine-Funktion fuer die D-16 Gesamt-Summe-Anzeige im Modal.

use crate::api::RepaymentEntryTO;

/// D-16: Gesamt-Auszahlungs-Summe in Cent.
///
/// Pure function: total = sum(entry.share_count_to_pay_out * share_value_cents).
/// Wird im Modal-Footer als "Summe: X €" angezeigt.
pub fn sum_payout_amounts(_entries: &[RepaymentEntryTO], _share_value_cents: i64) -> i64 {
    // TDD RED: stub returns wrong constant so tests fail.
    -1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::RepaymentEntryStatusTO;
    use uuid::Uuid;

    fn make_entry(share_count: i32) -> RepaymentEntryTO {
        RepaymentEntryTO {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: share_count,
            status: RepaymentEntryStatusTO::Open,
            created: None,
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn sum_single_entry() {
        assert_eq!(sum_payout_amounts(&[make_entry(1)], 10_000), 10_000);
        assert_eq!(sum_payout_amounts(&[make_entry(5)], 10_000), 50_000);
    }

    #[test]
    fn sum_multiple_entries() {
        let entries = vec![make_entry(2), make_entry(3), make_entry(1)];
        assert_eq!(sum_payout_amounts(&entries, 10_000), 60_000);
    }

    #[test]
    fn sum_empty_returns_zero() {
        assert_eq!(sum_payout_amounts(&[], 10_000), 0);
    }

    #[test]
    fn sum_zero_share_count_defensive() {
        assert_eq!(sum_payout_amounts(&[make_entry(0)], 10_000), 0);
    }

    #[test]
    fn sum_realistic_phase_total() {
        // 5 Mitglieder mit je 3 Anteilen, 100 EUR pro Anteil = 1.500 EUR
        let entries: Vec<_> = (0..5).map(|_| make_entry(3)).collect();
        assert_eq!(sum_payout_amounts(&entries, 10_000), 150_000); // 1500,00 EUR in Cent
    }
}
