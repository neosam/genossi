use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

/// Entity for the single original antrag file uploaded to an Application.
///
/// Deliberately narrow schema (Phase 25 CONTEXT decision #5): no domain
/// classifier and no free-form descriptor — the row's purpose is implicit
/// ("Original-Antrag") because at most one active row exists per application
/// (enforced by the partial unique index in the migration).
///
/// NOT auditable. This entity intentionally does NOT implement
/// `crate::auditable::Auditable` — the auditable copy is the `MemberDocument`
/// created during `confirm()` (Move / Ownership-Übergabe).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationDocumentEntity {
    pub id: Uuid,
    pub application_id: Uuid,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
    pub size: i64,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait ApplicationDocumentDao {
    type Transaction: crate::Transaction;

    /// Return every row in the table, including soft-deleted rows.
    /// Used by the default implementations of `all`, `find_by_id`, and
    /// `find_active_by_application_id`.
    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError>;

    /// Insert a new row. Callers must guarantee the single-slot invariant
    /// (no active row already exists for `entity.application_id`); a
    /// violation surfaces as `DaoError::DatabaseError` via the partial
    /// unique index.
    async fn create(
        &self,
        entity: &ApplicationDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Update an existing row using optimistic locking on `version`.
    /// Returns `DaoError::ConflictError` when the version mismatches
    /// (lost-update guard). Used for both content replacement and
    /// soft-delete (setting `deleted`).
    async fn update(
        &self,
        entity: &ApplicationDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active: Vec<ApplicationDocumentEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ApplicationDocumentEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    /// Return the single active row for `application_id`, if any.
    /// Returns `None` when no row exists or the only matching row has
    /// `deleted = Some(_)`. Single-slot invariant (unique partial index
    /// `WHERE deleted IS NULL`) guarantees at most one match.
    async fn find_active_by_application_id(
        &self,
        application_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ApplicationDocumentEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.application_id == application_id && e.deleted.is_none())
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockTransaction;

    fn make_entity(application_id: Uuid, deleted: bool) -> ApplicationDocumentEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 3).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        ApplicationDocumentEntity {
            id: Uuid::new_v4(),
            application_id,
            file_name: Arc::from("antrag_scan.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("applications/foo/bar.pdf"),
            size: 12345,
            created: datetime,
            deleted: if deleted { Some(datetime) } else { None },
            version: Uuid::new_v4(),
        }
    }

    /// Test-only fixture DAO: hard-coded `dump_all` payload; all other
    /// methods use the trait default impls, which is what we want to
    /// exercise. Mirrors the fixture approach used by
    /// `member_document::tests` (test-focused, not tied to the mockall
    /// mock which cannot cover default-method behaviour).
    struct FixtureDao {
        rows: Vec<ApplicationDocumentEntity>,
    }

    #[async_trait]
    impl ApplicationDocumentDao for FixtureDao {
        type Transaction = MockTransaction;

        async fn dump_all(
            &self,
            _tx: Self::Transaction,
        ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError> {
            Ok(Arc::from(self.rows.clone()))
        }

        async fn create(
            &self,
            _entity: &ApplicationDocumentEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            unimplemented!("fixture: not used in tests")
        }

        async fn update(
            &self,
            _entity: &ApplicationDocumentEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            unimplemented!("fixture: not used in tests")
        }
    }

    fn mock_tx() -> MockTransaction {
        let mut m = MockTransaction::new();
        m.expect_clone().returning(MockTransaction::new);
        m
    }

    #[tokio::test]
    async fn test_find_active_by_application_id_returns_active_row() {
        let app_id = Uuid::new_v4();
        let other_app_id = Uuid::new_v4();
        let target = make_entity(app_id, false);
        let target_id = target.id;
        let dao = FixtureDao {
            rows: vec![make_entity(other_app_id, false), target.clone()],
        };

        let found = dao
            .find_active_by_application_id(app_id, mock_tx())
            .await
            .expect("query succeeds");
        let found = found.expect("active row present");
        assert_eq!(found.id, target_id);
        assert_eq!(found.application_id, app_id);
    }

    #[tokio::test]
    async fn test_find_active_by_application_id_ignores_soft_deleted_row() {
        let app_id = Uuid::new_v4();
        // Only row for this application is soft-deleted → must return None.
        let dao = FixtureDao {
            rows: vec![make_entity(app_id, true)],
        };

        let found = dao
            .find_active_by_application_id(app_id, mock_tx())
            .await
            .expect("query succeeds");
        assert!(
            found.is_none(),
            "soft-deleted rows must not surface via find_active_by_application_id",
        );
    }

    #[tokio::test]
    async fn test_all_filters_soft_deleted() {
        let app_id = Uuid::new_v4();
        let dao = FixtureDao {
            rows: vec![make_entity(app_id, false), make_entity(app_id, true)],
        };
        let active = dao.all(mock_tx()).await.expect("all succeeds");
        assert_eq!(active.len(), 1);
    }
}
