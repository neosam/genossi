//! Repayment phases list page (Phase 12 Plan 12-04, UI-01) — admin-only.
//!
//! Pattern aus genossi-frontend/src/page/assemblies.rs (1:1-Klon mit
//! fiscal_year + share_value + Anzahl-Einträge statt name + date + location).
//! Status-Badges via RepaymentPhaseStatusBadge (Plan 12-02).
//! Default-Sort: fiscal_year DESC, created DESC (D-14).
//! Anzahl-Einträge pro Row via use_resource auf list_repayment_entries (UI-01 SC#1; N+1 akzeptabel <20 Phasen).
//! Euro-Parse via crate::component::repayment_format::parse_euro_to_cents (Plan 12-02 kanonisch).

use crate::api::RepaymentPhaseTO;

/// D-14 Claude's Discretion: Default-Sort `fiscal_year DESC, created DESC`
/// (Phase-7 D-08-Notiz: "Frontend (Phase 12) sortiert per `fiscal_year DESC,
/// created DESC` zur Auffindbarkeit").
///
/// Stable bei `fiscal_year + created`-Ties (Rust's sort_by ist stable).
///
/// TDD RED stub: liefert bewusst die unsortierte Eingabe — Test schlägt fehl,
/// bis Task 2 die echte Implementation einsetzt.
fn sort_phases_default(phases: &[RepaymentPhaseTO]) -> Vec<RepaymentPhaseTO> {
    phases.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{RepaymentPhaseStatusTO, RepaymentPhaseTO};
    use uuid::Uuid;

    fn make_phase(year: i32, created_iso: &str) -> RepaymentPhaseTO {
        RepaymentPhaseTO {
            id: Uuid::new_v4(),
            fiscal_year: year,
            share_value: 10_000,
            status: RepaymentPhaseStatusTO::Preparation,
            opened_at: None,
            closed_at: None,
            created: Some(created_iso.into()),
            deleted: None,
            version: None,
        }
    }

    #[test]
    fn sort_by_fiscal_year_desc() {
        let phases = vec![
            make_phase(2023, "2023-01-01T00:00:00Z"),
            make_phase(2025, "2025-01-01T00:00:00Z"),
            make_phase(2024, "2024-01-01T00:00:00Z"),
        ];
        let sorted = sort_phases_default(&phases);
        assert_eq!(sorted[0].fiscal_year, 2025);
        assert_eq!(sorted[1].fiscal_year, 2024);
        assert_eq!(sorted[2].fiscal_year, 2023);
    }

    #[test]
    fn sort_by_created_desc_within_same_year() {
        let phases = vec![
            make_phase(2025, "2025-01-15T00:00:00Z"),
            make_phase(2025, "2025-06-01T00:00:00Z"),
            make_phase(2025, "2025-03-01T00:00:00Z"),
        ];
        let sorted = sort_phases_default(&phases);
        assert_eq!(sorted[0].created.as_deref(), Some("2025-06-01T00:00:00Z"));
        assert_eq!(sorted[1].created.as_deref(), Some("2025-03-01T00:00:00Z"));
        assert_eq!(sorted[2].created.as_deref(), Some("2025-01-15T00:00:00Z"));
    }

    #[test]
    fn sort_empty_returns_empty() {
        let sorted = sort_phases_default(&[]);
        assert_eq!(sorted.len(), 0);
    }
}
