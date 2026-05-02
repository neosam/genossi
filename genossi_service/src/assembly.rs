//! Service-layer domain types and trait for the Assembly aggregate.
//!
//! Plan 03 wires the full `AssemblyService` trait, lifecycle methods, and
//! input DTOs (`AssemblySubmission`, `AssemblyUpdate`). Plan 02 previously
//! shipped only the minimum stub (`Assembly`, `AssemblyDetail`); we extend
//! that here without breaking the existing `From<&AssemblyEntity>` symmetry
//! that `genossi_rest_types` relies on.

use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyEntity, AssemblyStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Service-layer representation of an Assembly (Generalversammlung).
///
/// Mirrors `AssemblyEntity` from `genossi_dao::assembly` but uses `Arc<str>`
/// for string fields per the genossi_service convention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
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

impl From<&AssemblyEntity> for Assembly {
    fn from(entity: &AssemblyEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
            date: entity.date,
            location: entity.location.clone(),
            status: entity.status.clone(),
            opened_at: entity.opened_at,
            closed_at: entity.closed_at,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Assembly> for AssemblyEntity {
    fn from(a: &Assembly) -> Self {
        Self {
            id: a.id,
            name: a.name.clone(),
            date: a.date,
            location: a.location.clone(),
            status: a.status.clone(),
            opened_at: a.opened_at,
            closed_at: a.closed_at,
            created: a.created,
            deleted: a.deleted,
            version: a.version,
        }
    }
}

/// Input for creating a new assembly. The service sets status, opened_at,
/// closed_at, version, created automatically (D-11).
#[derive(Clone, Debug)]
pub struct AssemblySubmission {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
}

/// Input for updating an existing assembly. Only allowed in status
/// `Preparation` (D-07). `version` is mandatory (optimistic locking).
#[derive(Clone, Debug)]
pub struct AssemblyUpdate {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
    pub version: Uuid,
}

/// Service-layer detail object: an Assembly plus its snapshot member count.
///
/// Per RESEARCH §6 / Open Q1, the snapshot count is exposed ad-hoc (not the
/// full member id list), keeping the wire format minimal and avoiding PII
/// disclosure for read endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyDetail {
    pub assembly: Assembly,
    pub snapshot_member_count: u64,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AssemblyService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Create a new assembly in status `Preparation` (D-11). Audit-process
    /// `assembly.create`. Requires `admin` privilege.
    async fn create_assembly(
        &self,
        submission: &AssemblySubmission,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;

    /// Update the editable fields of an assembly. Allowed only in status
    /// `Preparation` (D-07). Requires matching version (optimistic locking).
    /// Audit-process `assembly.update`.
    async fn update_assembly(
        &self,
        id: Uuid,
        update: &AssemblyUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;

    /// Transition `Preparation` → `Open` (D-08). Atomically captures the
    /// member snapshot. Audit-process `assembly.open`. Requires `admin`.
    async fn open_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;

    /// Transition `Open` → `Closed` (D-09). NO HelperSession cascade in
    /// Phase 1 (Phase 3 will extend). Audit-process `assembly.close`.
    /// Requires `admin`.
    async fn close_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;

    /// Get a single assembly with snapshot count (RESEARCH §6).
    async fn get_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<AssemblyDetail, ServiceError>;

    /// List all non-deleted assemblies. Requires `admin`.
    async fn get_all_assemblies(
        &self,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[Assembly]>, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: Uuid::new_v4(),
            name: Arc::from("GV 2026"),
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
    fn entity_to_assembly_roundtrip_preserves_fields() {
        let entity = make_entity();
        let domain: Assembly = (&entity).into();
        let back: AssemblyEntity = (&domain).into();
        assert_eq!(back.id, entity.id);
        assert_eq!(back.name, entity.name);
        assert_eq!(back.status, entity.status);
        assert_eq!(back.version, entity.version);
    }

    #[test]
    fn assembly_detail_holds_count() {
        let entity = make_entity();
        let domain: Assembly = (&entity).into();
        let detail = AssemblyDetail {
            assembly: domain.clone(),
            snapshot_member_count: 17,
        };
        assert_eq!(detail.snapshot_member_count, 17);
        assert_eq!(detail.assembly.id, domain.id);
    }

    #[test]
    fn test_assembly_from_entity_roundtrip() {
        let entity = make_entity();
        let domain: Assembly = (&entity).into();
        let back: AssemblyEntity = (&domain).into();
        assert_eq!(back, entity);
    }

    #[test]
    fn test_mock_assembly_service_compiles() {
        // Compile-only: ensure #[automock] generated MockAssemblyService.
        let _: MockAssemblyService = MockAssemblyService::new();
    }

    #[test]
    fn test_assembly_submission_constructible() {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let submission = AssemblySubmission {
            name: Arc::from("GV 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
        };
        assert_eq!(&*submission.name, "GV 2026");
    }

    #[test]
    fn test_assembly_update_requires_version() {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let version = Uuid::new_v4();
        let update = AssemblyUpdate {
            name: Arc::from("GV 2026 (renamed)"),
            date: datetime,
            location: None,
            version,
        };
        assert_eq!(update.version, version);
    }
}
