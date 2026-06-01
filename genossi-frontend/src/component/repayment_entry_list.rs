//! Repayment-Entry-Liste (Phase 12 Plan 12-08, UI-03).
//!
//! 7-Spalten-Tabelle + Multi-Select + Status-Filter + Inline-Cell-Edit.
//! Component-First: reusen MemberSearch-Pattern (mail_page.rs), Modal,
//! ToastContainer, EditableShareCountCell, RepaymentEntryStatusBadge,
//! format_payout_eur.
//!
//! Default-Sort: Mitgliedsnummer ASC, created ASC (D-14).
//! Status-Filter ist client-side (Backend liefert immer alle, Phase 8 D-10).
//!
//! Wave 1 (Task 1): Pure-Helper-Funktionen + Unit-Tests (TDD-RED-Phase Stubs).
//! Wave 2 (Task 2): Vollstaendige RepaymentEntryList-Component.

use crate::api::{RepaymentEntryStatusTO, RepaymentEntryTO};
use rest_types::MemberTO;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Open,
    Contacted,
    PaidOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCounts {
    pub all: usize,
    pub open: usize,
    pub contacted: usize,
    pub paidout: usize,
}

// ─── Pure helpers ────────────────────────────────────────────────────

/// D-12: Client-side Filter — Backend liefert immer alle Entries (Phase 8 D-10).
pub fn filter_entries_by_status(
    entries: &[RepaymentEntryTO],
    filter: StatusFilter,
) -> Vec<RepaymentEntryTO> {
    entries
        .iter()
        .filter(|e| match filter {
            StatusFilter::All => true,
            StatusFilter::Open => matches!(e.status, RepaymentEntryStatusTO::Open),
            StatusFilter::Contacted => matches!(e.status, RepaymentEntryStatusTO::Contacted),
            StatusFilter::PaidOut => matches!(e.status, RepaymentEntryStatusTO::PaidOut),
        })
        .cloned()
        .collect()
}

/// D-12: Count-Badges fuer die Tab-Strip-im-Tab Status-Filter.
pub fn entry_counts_by_status(entries: &[RepaymentEntryTO]) -> StatusCounts {
    let mut c = StatusCounts {
        all: entries.len(),
        open: 0,
        contacted: 0,
        paidout: 0,
    };
    for e in entries {
        match e.status {
            RepaymentEntryStatusTO::Open => c.open += 1,
            RepaymentEntryStatusTO::Contacted => c.contacted += 1,
            RepaymentEntryStatusTO::PaidOut => c.paidout += 1,
        }
    }
    c
}

/// Client-Side-Join Member ↔ Entry via MEMBERS-Global-Signal (D-10).
/// Bei Member-Mismatch (member_id nicht in MEMBERS-Liste) liefert None —
/// Caller rendert dann "—" als defensive UX (Pitfall 8).
pub fn member_for_entry<'a>(
    entry: &RepaymentEntryTO,
    members: &'a [MemberTO],
) -> Option<&'a MemberTO> {
    members.iter().find(|m| m.id == Some(entry.member_id))
}

/// D-14 Default-Sort: Mitgliedsnummer ASC, created ASC sekundaer.
/// Entries ohne Member-Match (member_for_entry == None) sortieren ans Ende.
pub fn sort_entries_default(
    entries: &[RepaymentEntryTO],
    members: &[MemberTO],
) -> Vec<RepaymentEntryTO> {
    let mut result: Vec<RepaymentEntryTO> = entries.to_vec();
    result.sort_by(|a, b| {
        let ma = member_for_entry(a, members);
        let mb = member_for_entry(b, members);
        match (ma, mb) {
            (Some(a_m), Some(b_m)) => a_m
                .member_number
                .cmp(&b_m.member_number)
                .then_with(|| a.created.cmp(&b.created)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.created.cmp(&b.created),
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rest_types::MemberStatusTO;
    use time::macros::date;
    use uuid::Uuid;

    /// Option A (Issue #3): MemberTO hat KEIN #[derive(Default)]
    /// (verifiziert via `rg "impl Default for MemberTO|#\[derive\([^)]*Default" rest-types/src/lib.rs` → 0).
    /// Daher listet make_member ALLE Felder explizit auf — KEIN ..Default::default()-Spread.
    /// Wenn das Backend-Schema waechst (neues Feld), bricht der Compiler hier — bewusste Pflicht-Sync.
    fn make_member(id: Uuid, number: i64) -> MemberTO {
        MemberTO {
            id: Some(id),
            member_number: number,
            first_name: format!("First{number}"),
            last_name: format!("Last{number}"),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date!(2020 - 01 - 01),
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: MemberStatusTO::Normal,
            created: None,
            deleted: None,
            version: None,
        }
    }

    fn make_entry(
        id: Uuid,
        member_id: Uuid,
        status: RepaymentEntryStatusTO,
        created_iso: &str,
    ) -> RepaymentEntryTO {
        RepaymentEntryTO {
            id,
            member_id,
            phase_id: Uuid::new_v4(),
            share_count_to_pay_out: 1,
            status,
            created: Some(created_iso.into()),
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn filter_by_status_open() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Contacted,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-03T00:00:00Z",
        );
        let filtered = filter_entries_by_status(&[e1, e2, e3], StatusFilter::Open);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_status_all_returns_all() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::PaidOut,
            "2026-01-02T00:00:00Z",
        );
        let filtered = filter_entries_by_status(&[e1, e2], StatusFilter::All);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn counts_correct() {
        let e1 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Contacted,
            "2026-01-03T00:00:00Z",
        );
        let e4 = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::PaidOut,
            "2026-01-04T00:00:00Z",
        );
        let c = entry_counts_by_status(&[e1, e2, e3, e4]);
        assert_eq!(c.all, 4);
        assert_eq!(c.open, 2);
        assert_eq!(c.contacted, 1);
        assert_eq!(c.paidout, 1);
    }

    #[test]
    fn counts_empty_returns_zeros() {
        let c = entry_counts_by_status(&[]);
        assert_eq!(c.all, 0);
        assert_eq!(c.open, 0);
        assert_eq!(c.contacted, 0);
        assert_eq!(c.paidout, 0);
    }

    #[test]
    fn member_for_entry_finds_match() {
        let mid = Uuid::new_v4();
        let m = make_member(mid, 42);
        let e = make_entry(
            Uuid::new_v4(),
            mid,
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(
            member_for_entry(&e, &[m]).map(|m| m.member_number),
            Some(42)
        );
    }

    #[test]
    fn member_for_entry_returns_none_on_mismatch() {
        let m = make_member(Uuid::new_v4(), 42);
        let e = make_entry(
            Uuid::new_v4(),
            Uuid::new_v4(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        assert!(member_for_entry(&e, &[m]).is_none());
    }

    #[test]
    fn sort_by_member_number_asc() {
        let m1 = make_member(Uuid::new_v4(), 100);
        let m2 = make_member(Uuid::new_v4(), 50);
        let m3 = make_member(Uuid::new_v4(), 75);
        let e1 = make_entry(
            Uuid::new_v4(),
            m1.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            m2.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let e3 = make_entry(
            Uuid::new_v4(),
            m3.id.unwrap(),
            RepaymentEntryStatusTO::Open,
            "2026-01-03T00:00:00Z",
        );
        let members = vec![m1.clone(), m2.clone(), m3.clone()];
        let sorted = sort_entries_default(&[e1, e2, e3], &members);
        assert_eq!(
            member_for_entry(&sorted[0], &members)
                .unwrap()
                .member_number,
            50
        );
        assert_eq!(
            member_for_entry(&sorted[1], &members)
                .unwrap()
                .member_number,
            75
        );
        assert_eq!(
            member_for_entry(&sorted[2], &members)
                .unwrap()
                .member_number,
            100
        );
    }

    #[test]
    fn sort_entries_without_member_at_end() {
        let m1 = make_member(Uuid::new_v4(), 50);
        let m1_id = m1.id.unwrap();
        let unknown_member_id = Uuid::new_v4();
        let e1 = make_entry(
            Uuid::new_v4(),
            unknown_member_id,
            RepaymentEntryStatusTO::Open,
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            Uuid::new_v4(),
            m1_id,
            RepaymentEntryStatusTO::Open,
            "2026-01-02T00:00:00Z",
        );
        let sorted = sort_entries_default(&[e1, e2], &[m1]);
        // Member-matched first, unmatched last
        assert_eq!(sorted[0].member_id, m1_id);
        assert_eq!(sorted[1].member_id, unknown_member_id);
    }
}
