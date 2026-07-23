use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

/// Entity for an inline mail image asset (Phase 27, IMG-01).
///
/// Diverges from the `application_document` analog: bytes are stored **inline
/// as a SQLite BLOB** (`bytes: Vec<u8>`), NOT on the filesystem. There is no
/// parent entity id (mail assets stand alone) and no single-slot invariant.
///
/// NOT auditable. This entity intentionally does NOT implement
/// `crate::auditable::Auditable` — mail assets are non-core (IMG-01, analog to
/// the Application-Doc pattern for non-audited entities).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAssetEntity {
    pub id: Uuid,
    pub filename: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub bytes: Vec<u8>,
    pub uploaded_by: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MailAssetDao {
    type Transaction: crate::Transaction;

    /// Return every row in the table, including soft-deleted rows.
    /// Used by the default implementations of `all` and `find_by_id`.
    async fn dump_all(&self, tx: Self::Transaction)
        -> Result<Arc<[MailAssetEntity]>, DaoError>;

    /// Insert a new row.
    async fn create(
        &self,
        entity: &MailAssetEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// Update an existing row using optimistic locking on `version`.
    /// Returns `DaoError::ConflictError` when the version mismatches
    /// (lost-update guard). Used for soft-delete (setting `deleted`).
    async fn update(
        &self,
        entity: &MailAssetEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MailAssetEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active: Vec<MailAssetEntity> = all_entities
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
    ) -> Result<Option<MailAssetEntity>, DaoError> {
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
    use crate::MockTransaction;

    fn make_entity(deleted: bool) -> MailAssetEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 23).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MailAssetEntity {
            id: Uuid::new_v4(),
            filename: Arc::from("logo.png"),
            mime_type: Arc::from("image/png"),
            size_bytes: 4,
            bytes: vec![0x89, 0x50, 0x4E, 0x47],
            uploaded_by: Arc::from("admin-user"),
            created: datetime,
            deleted: if deleted { Some(datetime) } else { None },
            version: Uuid::new_v4(),
        }
    }

    /// Test-only fixture DAO: hard-coded `dump_all` payload; all other methods
    /// use the trait default impls, which is what we want to exercise.
    struct FixtureDao {
        rows: Vec<MailAssetEntity>,
    }

    #[async_trait]
    impl MailAssetDao for FixtureDao {
        type Transaction = MockTransaction;

        async fn dump_all(
            &self,
            _tx: Self::Transaction,
        ) -> Result<Arc<[MailAssetEntity]>, DaoError> {
            Ok(Arc::from(self.rows.clone()))
        }

        async fn create(
            &self,
            _entity: &MailAssetEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            unimplemented!("fixture: not used in tests")
        }

        async fn update(
            &self,
            _entity: &MailAssetEntity,
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
    async fn test_all_filters_soft_deleted() {
        let dao = FixtureDao {
            rows: vec![make_entity(false), make_entity(true)],
        };
        let active = dao.all(mock_tx()).await.expect("all succeeds");
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_id_ignores_soft_deleted() {
        let deleted = make_entity(true);
        let deleted_id = deleted.id;
        let dao = FixtureDao {
            rows: vec![deleted],
        };
        let found = dao
            .find_by_id(deleted_id, mock_tx())
            .await
            .expect("query succeeds");
        assert!(
            found.is_none(),
            "soft-deleted rows must not surface via find_by_id",
        );
    }
}
