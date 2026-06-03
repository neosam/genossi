use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RepaymentPhaseStatus {
    #[default]
    Preparation,
    Open,
    Closed,
}

impl RepaymentPhaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepaymentPhaseStatus::Preparation => "Preparation",
            RepaymentPhaseStatus::Open => "Open",
            RepaymentPhaseStatus::Closed => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Preparation" => Ok(RepaymentPhaseStatus::Preparation),
            "Open" => Ok(RepaymentPhaseStatus::Open),
            "Closed" => Ok(RepaymentPhaseStatus::Closed),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown repayment phase status: {}",
                s
            )))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentPhaseEntity {
    pub id: Uuid,
    pub fiscal_year: i32,
    pub share_value: i64,
    pub status: RepaymentPhaseStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl crate::auditable::Auditable for RepaymentPhaseEntity {
    fn entity_type() -> &'static str {
        "repayment_phase"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        // WR-08: do NOT use `unwrap_or_default()` here -- a silent empty
        // string in the audit log is forensically useless. The hash chain
        // would still be intact, but auditors would see "" instead of the
        // intended timestamp. Log the failure and substitute a sentinel
        // string so the breakage is at least visible.
        let format_dt = |dt: &time::PrimitiveDateTime| {
            dt.assume_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|err| {
                    tracing::error!(
                        error = ?err,
                        entity = "repayment_phase",
                        "Failed to format datetime for audit field"
                    );
                    "<invalid datetime>".to_string()
                })
        };
        vec![
            ("fiscal_year", Some(self.fiscal_year.to_string())),
            ("share_value", Some(self.share_value.to_string())),
            ("status", Some(self.status.as_str().to_string())),
            ("opened_at", self.opened_at.as_ref().map(format_dt)),
            ("closed_at", self.closed_at.as_ref().map(format_dt)),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait RepaymentPhaseDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &RepaymentPhaseEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &RepaymentPhaseEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<RepaymentPhaseEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<RepaymentPhaseEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;

    fn make_repayment_phase() -> RepaymentPhaseEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 29).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000, // 120,00 EUR in Cent
            status: RepaymentPhaseStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_repayment_phase_status_roundtrip() {
        for variant in [
            RepaymentPhaseStatus::Preparation,
            RepaymentPhaseStatus::Open,
            RepaymentPhaseStatus::Closed,
        ] {
            let s = variant.as_str();
            let parsed = RepaymentPhaseStatus::from_str(s).expect("roundtrip parses");
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_repayment_phase_status_strings_are_english() {
        // D-01: All status strings must be English (pattern-consistent with
        // AssemblyStatus/MemberStatus/ApplicationStatus); frontend translates
        // via i18n.
        assert_eq!(RepaymentPhaseStatus::Preparation.as_str(), "Preparation");
        assert_eq!(RepaymentPhaseStatus::Open.as_str(), "Open");
        assert_eq!(RepaymentPhaseStatus::Closed.as_str(), "Closed");
        // Round-trip via the literal English string must succeed.
        assert_eq!(
            RepaymentPhaseStatus::from_str("Preparation").unwrap(),
            RepaymentPhaseStatus::Preparation
        );
    }

    #[test]
    fn test_repayment_phase_status_invalid_string() {
        // D-01 / T-07-01-05: German "Vorbereitung" must NOT parse — only the
        // three English strings are accepted.
        let err = RepaymentPhaseStatus::from_str("Vorbereitung");
        assert!(matches!(err, Err(DaoError::ParseError(_))));

        let err2 = RepaymentPhaseStatus::from_str("");
        assert!(matches!(err2, Err(DaoError::ParseError(_))));
    }

    #[test]
    fn test_repayment_phase_status_default_is_preparation() {
        assert_eq!(
            RepaymentPhaseStatus::default(),
            RepaymentPhaseStatus::Preparation
        );
    }

    #[test]
    fn test_auditable_entity_type_is_repayment_phase() {
        assert_eq!(RepaymentPhaseEntity::entity_type(), "repayment_phase");
    }

    #[test]
    fn test_auditable_fields_count_and_excludes_metadata() {
        let entity = make_repayment_phase();
        let fields = entity.audit_fields();
        assert_eq!(
            fields.len(),
            5,
            "audit_fields must contain exactly 5 entries (fiscal_year, share_value, status, opened_at, closed_at)"
        );

        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            field_names,
            vec![
                "fiscal_year",
                "share_value",
                "status",
                "opened_at",
                "closed_at"
            ]
        );

        // Lifecycle / metadata fields MUST NOT be in audit_fields
        // (Auditable convention — see assembly.rs Z. 67-93):
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_status_change() {
        let old = make_repayment_phase();
        let mut new = old.clone();
        new.status = RepaymentPhaseStatus::Open;

        let changes = old.diff(&new);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert_eq!(changes.len(), 1, "exactly one field changed (status)");
        assert_eq!(names, vec!["status"]);
    }
}
