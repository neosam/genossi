use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyStatus {
    Preparation,
    Open,
    Closed,
}

impl AssemblyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssemblyStatus::Preparation => "Preparation",
            AssemblyStatus::Open => "Open",
            AssemblyStatus::Closed => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Preparation" => Ok(AssemblyStatus::Preparation),
            "Open" => Ok(AssemblyStatus::Open),
            "Closed" => Ok(AssemblyStatus::Closed),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown assembly status: {}",
                s
            )))),
        }
    }
}

impl Default for AssemblyStatus {
    fn default() -> Self {
        AssemblyStatus::Preparation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyEntity {
    pub id: Uuid,
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
    pub status: AssemblyStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl crate::auditable::Auditable for AssemblyEntity {
    fn entity_type() -> &'static str {
        "assembly"
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
                        entity = "assembly",
                        "Failed to format datetime for audit field"
                    );
                    "<invalid datetime>".to_string()
                })
        };
        vec![
            ("name", Some(self.name.to_string())),
            ("date", Some(format_dt(&self.date))),
            ("location", self.location.as_ref().map(|s| s.to_string())),
            ("status", Some(self.status.as_str().to_string())),
            ("opened_at", self.opened_at.as_ref().map(format_dt)),
            ("closed_at", self.closed_at.as_ref().map(format_dt)),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &AssemblyEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &AssemblyEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<AssemblyEntity> = all_entities
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
    ) -> Result<Option<AssemblyEntity>, DaoError> {
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

    fn make_assembly() -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: Uuid::new_v4(),
            name: Arc::from("Generalversammlung 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
            status: AssemblyStatus::Preparation,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_assembly_status_roundtrip() {
        for variant in [
            AssemblyStatus::Preparation,
            AssemblyStatus::Open,
            AssemblyStatus::Closed,
        ] {
            let s = variant.as_str();
            let parsed = AssemblyStatus::from_str(s).expect("roundtrip parses");
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_assembly_status_strings_are_english() {
        // D-06, D-17: All status strings must be English.
        assert_eq!(AssemblyStatus::Preparation.as_str(), "Preparation");
        assert_eq!(AssemblyStatus::Open.as_str(), "Open");
        assert_eq!(AssemblyStatus::Closed.as_str(), "Closed");
        // Pitfall 4: round-trip via the literal English string must succeed.
        assert_eq!(
            AssemblyStatus::from_str("Preparation").unwrap(),
            AssemblyStatus::Preparation
        );
    }

    #[test]
    fn test_assembly_status_invalid_string() {
        // German "Vorbereitung" must NOT parse — D-06/D-17 require English.
        let err = AssemblyStatus::from_str("Vorbereitung");
        assert!(matches!(err, Err(DaoError::ParseError(_))));

        let err2 = AssemblyStatus::from_str("");
        assert!(matches!(err2, Err(DaoError::ParseError(_))));
    }

    #[test]
    fn test_assembly_status_default_is_preparation() {
        assert_eq!(AssemblyStatus::default(), AssemblyStatus::Preparation);
    }

    #[test]
    fn test_auditable_entity_type_is_assembly() {
        assert_eq!(AssemblyEntity::entity_type(), "assembly");
    }

    #[test]
    fn test_auditable_fields_count_and_excludes() {
        let entity = make_assembly();
        let fields = entity.audit_fields();
        assert_eq!(
            fields.len(),
            6,
            "audit_fields must contain exactly 6 entries (D-10)"
        );

        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            field_names,
            vec![
                "name",
                "date",
                "location",
                "status",
                "opened_at",
                "closed_at"
            ]
        );

        // Lifecycle fields MUST NOT be in audit_fields:
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_status_change() {
        let old = make_assembly();
        let mut new = old.clone();
        new.status = AssemblyStatus::Open;
        new.opened_at = Some(old.date);

        let changes = old.diff(&new);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert!(names.contains(&"status"));
        assert!(names.contains(&"opened_at"));
        assert_eq!(changes.len(), 2);
    }
}
