use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyMemberSnapshotEntity {
    pub assembly_id: Uuid,
    pub member_id: Uuid,
    pub captured_at: time::PrimitiveDateTime,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyMemberSnapshotDao {
    type Transaction: crate::Transaction;

    async fn create(
        &self,
        entity: &AssemblyMemberSnapshotEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn create_batch(
        &self,
        entities: &[AssemblyMemberSnapshotEntity],
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn find_by_assembly_id(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[AssemblyMemberSnapshotEntity]>, DaoError>;

    async fn count_by_assembly_id(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_entity_has_three_fields_only() {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let entity = AssemblyMemberSnapshotEntity {
            assembly_id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            captured_at: datetime,
        };
        // Smoke test: the struct has exactly the three fields we expect.
        // If a future change adds id/version/created/deleted, this test won't fail
        // automatically — but constructing the struct here serves as a compile-time
        // contract (Pitfall 1: snapshot is data, not lifecycle).
        assert_eq!(entity.captured_at, datetime);
    }
}
