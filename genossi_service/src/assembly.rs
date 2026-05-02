//! Service-layer domain types for the Assembly aggregate.
//!
//! Phase 1, Plan 03 will fill in the full `AssemblyService` trait, lifecycle
//! methods, validation, and audit-process strings. This module currently
//! exposes only the minimum domain objects required by `genossi_rest_types`
//! (Plan 02): `Assembly`, `AssemblyDetail`, plus the bidirectional
//! `From<AssemblyEntity>` conversions that keep service↔DAO symmetry.

use genossi_dao::assembly::{AssemblyEntity, AssemblyStatus};
use std::sync::Arc;
use uuid::Uuid;

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
}
