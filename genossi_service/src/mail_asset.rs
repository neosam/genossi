//! Service trait for inline mail image assets (Phase 27, IMG-01/02/04).
//!
//! Three admin-only operations:
//!
//! - `upload` — magic-byte-sniffs the payload (PNG/JPEG/GIF only), enforces the
//!   5 MB limit, stores the bytes inline as a SQLite BLOB, returns the domain
//!   `MailAsset` with the SERVER-derived MIME (client MIME is untrusted).
//! - `download` — combined DB lookup returning `(metadata, bytes)` read inline
//!   from the entity (no filesystem load).
//! - `get` — metadata-only lookup.
//!
//! CR-02: every method calls `check_permission("admin")` as the FIRST
//! statement, before any DAO call or side effect. The impl enforces this; a
//! regression-guard unit test pins the ordering (zero DAO calls on denial).

use async_trait::async_trait;
use genossi_dao::mail_asset::MailAssetEntity;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Service-layer view of a mail asset. Mirrors [`MailAssetEntity`], excluding
/// the raw `bytes` (returned separately by `download`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAsset {
    pub id: Uuid,
    pub filename: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub uploaded_by: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&MailAssetEntity> for MailAsset {
    fn from(entity: &MailAssetEntity) -> Self {
        Self {
            id: entity.id,
            filename: entity.filename.clone(),
            mime_type: entity.mime_type.clone(),
            size_bytes: entity.size_bytes,
            uploaded_by: entity.uploaded_by.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

/// Input DTO for the REST → Service upload call.
///
/// `mime_type` carries the client-declared value but is IGNORED by the service
/// — the stored MIME is derived from a magic-byte sniff of `data` (IMG-05
/// security intent: client Content-Type/extension is spoofable). `data` is
/// bounded at the REST layer by `DefaultBodyLimit`; the service enforces the
/// 5 MB limit defensively.
#[derive(Clone, Debug)]
pub struct UploadMailAsset {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MailAssetService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Admin-only upload. Rejects non-PNG/JPEG/GIF payloads (415 at REST) and
    /// payloads exceeding 5 MB (validation error). Stores the SERVER-derived
    /// MIME.
    async fn upload(
        &self,
        upload: UploadMailAsset,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MailAsset, ServiceError>;

    /// Admin-only bytes download. Returns `EntityNotFound` when the asset is
    /// missing or soft-deleted. Bytes are read inline from the entity.
    async fn download(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MailAsset, Vec<u8>), ServiceError>;

    /// Admin-only metadata lookup. Returns `EntityNotFound` when missing.
    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MailAsset, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_datetime() -> time::PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 23).unwrap();
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
    }

    fn make_entity() -> MailAssetEntity {
        MailAssetEntity {
            id: Uuid::new_v4(),
            filename: Arc::from("logo.png"),
            mime_type: Arc::from("image/png"),
            size_bytes: 4,
            bytes: vec![0x89, 0x50, 0x4E, 0x47],
            uploaded_by: Arc::from("admin-user"),
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_entity_to_service_preserves_fields() {
        let e = make_entity();
        let asset = MailAsset::from(&e);
        assert_eq!(asset.id, e.id);
        assert_eq!(asset.filename.as_ref(), e.filename.as_ref());
        assert_eq!(asset.mime_type.as_ref(), e.mime_type.as_ref());
        assert_eq!(asset.size_bytes, e.size_bytes);
        assert_eq!(asset.uploaded_by.as_ref(), e.uploaded_by.as_ref());
        assert_eq!(asset.version, e.version);
        assert_eq!(asset.deleted, e.deleted);
    }
}
