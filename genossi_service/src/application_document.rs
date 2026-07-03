//! Service trait for the single-slot application document (Original-Antrag).
//!
//! Phase 25 Wave 2 (Plan 25-03). The trait exposes four operations:
//!
//! - `upload` — creates a new active row OR replaces an existing one in place
//!   (save-new → update-DB → delete-old-best-effort). Enforces the single-slot
//!   invariant via `find_active_by_application_id` at the service layer; the
//!   DAO partial unique index (Plan 25-02) is the belt-and-suspenders.
//! - `get` — metadata lookup; returns `Ok(None)` when no active row exists.
//! - `download` — combined DB lookup + storage load, returning `(metadata, bytes)`.
//! - `delete` — soft-delete the row and best-effort remove the file (delete
//!   failure warns only; the DB truth wins).
//!
//! CR-02 (APDOC-02): every method must call
//! `check_permission(MANAGE_MEMBERS_PRIVILEGE)` BEFORE `current_user_id()`.
//! The impl enforces this; unit Test 3 there pins the ordering as a
//! regression guard.
//!
//! MIME allow-list + body limit are REUSED from `genossi_service::member_document`.
//! No duplicate allow-list here.

use async_trait::async_trait;
use genossi_dao::application_document::ApplicationDocumentEntity;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

/// Service-layer view of a single-slot application document.
///
/// Mirrors [`ApplicationDocumentEntity`] field-for-field; the only difference
/// is that string/Arc<str> boundaries stay identical (the entity already uses
/// `Arc<str>` for text columns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationDocument {
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

impl From<&ApplicationDocumentEntity> for ApplicationDocument {
    fn from(entity: &ApplicationDocumentEntity) -> Self {
        Self {
            id: entity.id,
            application_id: entity.application_id,
            file_name: entity.file_name.clone(),
            mime_type: entity.mime_type.clone(),
            relative_path: entity.relative_path.clone(),
            size: entity.size,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&ApplicationDocument> for ApplicationDocumentEntity {
    fn from(doc: &ApplicationDocument) -> Self {
        Self {
            id: doc.id,
            application_id: doc.application_id,
            file_name: doc.file_name.clone(),
            mime_type: doc.mime_type.clone(),
            relative_path: doc.relative_path.clone(),
            size: doc.size,
            created: doc.created,
            deleted: doc.deleted,
            version: doc.version,
        }
    }
}

/// Input DTO for the REST → Service call.
///
/// `mime_type` is server-derived at the REST layer via `lookup_allowed_mime`
/// against the file extension (T-25-03-03 mitigation: client-declared MIME is
/// discarded). `data` is bounded at the REST layer as well; the service
/// enforces `MAX_FILE_SIZE` defensively.
#[derive(Clone, Debug)]
pub struct UploadApplicationDocument {
    pub application_id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait ApplicationDocumentService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Create-new OR replace-in-place upload.
    ///
    /// If no active row exists for `upload.application_id`, this creates a new
    /// row + saves the file. Otherwise it saves the new file, updates the row
    /// in place (bumping `version`), and best-effort deletes the old file.
    /// The single-slot invariant is enforced by both the service branch and
    /// the DB partial unique index (defense-in-depth).
    async fn upload(
        &self,
        upload: UploadApplicationDocument,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ApplicationDocument, ServiceError>;

    /// Metadata lookup. Returns `Ok(None)` when no active row exists; this is
    /// NOT an error because the "no document yet" state is normal for an
    /// unconfirmed application.
    async fn get(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ApplicationDocument>, ServiceError>;

    /// Combined DB lookup + storage load. Returns `EntityNotFound` when no
    /// active row exists, and `InternalError` when the DB row exists but the
    /// file has vanished from storage (signals corruption, not "missing" —
    /// the caller must escalate).
    async fn download(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(ApplicationDocument, Vec<u8>), ServiceError>;

    /// Soft-delete the row (set `deleted`, bump `version`). Physical file
    /// removal is best-effort: a storage failure is logged and swallowed
    /// because the DB truth is the source of truth for the domain.
    async fn delete(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_datetime() -> time::PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 3).unwrap();
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
    }

    fn make_entity() -> ApplicationDocumentEntity {
        ApplicationDocumentEntity {
            id: Uuid::new_v4(),
            application_id: Uuid::new_v4(),
            file_name: Arc::from("antrag.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("applications/foo/bar.pdf"),
            size: 4096,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_entity_to_service_roundtrip_preserves_fields() {
        let e = make_entity();
        let doc = ApplicationDocument::from(&e);
        assert_eq!(doc.id, e.id);
        assert_eq!(doc.application_id, e.application_id);
        assert_eq!(doc.file_name.as_ref(), e.file_name.as_ref());
        assert_eq!(doc.mime_type.as_ref(), e.mime_type.as_ref());
        assert_eq!(doc.relative_path.as_ref(), e.relative_path.as_ref());
        assert_eq!(doc.size, e.size);
        assert_eq!(doc.version, e.version);
        assert_eq!(doc.deleted, e.deleted);

        let back: ApplicationDocumentEntity = (&doc).into();
        assert_eq!(back, e);
    }

    #[test]
    fn test_service_to_entity_conversion_preserves_soft_delete() {
        let mut e = make_entity();
        e.deleted = Some(make_test_datetime());
        let doc = ApplicationDocument::from(&e);
        let back: ApplicationDocumentEntity = (&doc).into();
        assert_eq!(back.deleted, e.deleted);
    }
}
