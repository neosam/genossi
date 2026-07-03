//! Impl for [`ApplicationDocumentService`] (Phase 25 Wave 2 / Plan 25-03).
//!
//! CR-02 (APDOC-02): every method calls `check_permission` BEFORE
//! `current_user_id`. Test 3 in the `tests` submodule below is the regression
//! guard; an `awk` gate in the plan's `<verify>` block also enforces this at
//! CI time.
//!
//! Storage layout: `applications/{app_id}/{doc_id}.{ext}` (Pitfall 7). The
//! extension is derived server-side via the same helper as
//! `member_document`; the MIME allow-list is enforced at the REST layer via
//! `genossi_service::member_document::lookup_allowed_mime` (defense in depth
//! at Wave 3).
//!
//! Replace-in-place sequence (Wave 2 Decision #2):
//! `storage.save(new_path)` → `dao.update(entity)` → `storage.delete(old_path)`
//! with the delete step being best-effort (warn-log on error, never
//! propagated). If the old file lingers, the DB truth wins; the file becomes
//! an orphan reachable only by UUID.

use async_trait::async_trait;
use genossi_dao::application::ApplicationDao;
use genossi_dao::application_document::ApplicationDocumentDao;
use genossi_dao::TransactionDao;
use genossi_service::application_document::{
    ApplicationDocument, ApplicationDocumentService, UploadApplicationDocument,
};
use genossi_service::document_storage::{DocumentStorage, StorageError};
// MIME allow-list + extension whitelist are enforced at the REST layer
// (Wave 3) via `genossi_service::member_document::{allowed_extensions,
// lookup_allowed_mime}`. Not imported here to avoid an unused-warning; the
// service enforces MAX_FILE_SIZE and application-exists only.
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

/// Process tag stored in DAO write calls (required by the trait even though
/// this entity is NOT audited).
const APPLICATION_DOCUMENT_PROCESS: &str = "app-doc-service";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB — mirrors MemberDocument.

gen_service_impl! {
    struct ApplicationDocumentServiceImpl: ApplicationDocumentService = ApplicationDocumentServiceDeps {
        ApplicationDocumentDao: ApplicationDocumentDao<Transaction = Self::Transaction> = application_document_dao,
        ApplicationDao: ApplicationDao<Transaction = Self::Transaction> = application_dao,
        DocumentStorage: DocumentStorage = document_storage,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

/// Extract and validate the file extension (duplicated verbatim from
/// `member_document.rs` per CLAUDE.md — small helpers stay in their own
/// crate).
fn extract_extension(file_name: &str) -> Option<String> {
    let ext = file_name.rsplit('.').next()?;
    if ext == file_name {
        return None;
    }
    if ext.is_empty() || ext.len() > 10 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

fn now_primitive() -> time::PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(now.date(), now.time())
}

#[async_trait]
impl<Deps: ApplicationDocumentServiceDeps> ApplicationDocumentService
    for ApplicationDocumentServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn upload(
        &self,
        upload: UploadApplicationDocument,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ApplicationDocument, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // CR-02 (APDOC-02): check_permission FIRST — no side effects allowed
        // before the caller is authorised.
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        // Then capture the user id (unused for audit here — this entity is
        // not audited — but retained for future observability parity with
        // member_document).
        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        // Validate file size.
        if upload.data.len() > MAX_FILE_SIZE {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("file"),
                message: Arc::from("File size exceeds 50 MB limit"),
            }]));
        }

        // Verify the application exists (defense against orphaned upload
        // pointing at a bogus application_id).
        self.application_dao
            .find_by_id(upload.application_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(upload.application_id))?;

        // Single-slot branch: look up any active row for this application.
        let existing = self
            .application_document_dao
            .find_active_by_application_id(upload.application_id, tx.clone())
            .await?;

        let doc_id = self.uuid_service.new_v4().await;
        let extension = extract_extension(&upload.file_name).unwrap_or_else(|| "bin".to_string());
        let new_relative_path = format!(
            "applications/{}/{}.{}",
            upload.application_id, doc_id, extension
        );

        let created_ts = now_primitive();
        let size = upload.data.len() as i64;
        let new_version = self.uuid_service.new_v4().await;

        match existing {
            None => {
                // Create-new path: build a fresh entity, insert, then persist
                // bytes. If storage.save fails after the insert, the outer
                // transaction rollback removes the DB row.
                let new_entity = genossi_dao::application_document::ApplicationDocumentEntity {
                    id: doc_id,
                    application_id: upload.application_id,
                    file_name: Arc::from(upload.file_name.as_str()),
                    mime_type: Arc::from(upload.mime_type.as_str()),
                    relative_path: Arc::from(new_relative_path.as_str()),
                    size,
                    created: created_ts,
                    deleted: None,
                    version: new_version,
                };

                self.application_document_dao
                    .create(&new_entity, APPLICATION_DOCUMENT_PROCESS, tx.clone())
                    .await?;

                self.document_storage
                    .save(&new_relative_path, &upload.data)
                    .await
                    .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

                self.transaction_dao.commit(tx).await?;
                Ok(ApplicationDocument::from(&new_entity))
            }
            Some(old) => {
                // Replace-in-place: save-new → update-DB → best-effort
                // delete-old (Decision #2).
                self.document_storage
                    .save(&new_relative_path, &upload.data)
                    .await
                    .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

                let updated_entity = genossi_dao::application_document::ApplicationDocumentEntity {
                    id: old.id,
                    application_id: old.application_id,
                    file_name: Arc::from(upload.file_name.as_str()),
                    mime_type: Arc::from(upload.mime_type.as_str()),
                    relative_path: Arc::from(new_relative_path.as_str()),
                    size,
                    created: old.created,
                    deleted: None,
                    version: new_version,
                };

                self.application_document_dao
                    .update(&updated_entity, APPLICATION_DOCUMENT_PROCESS, tx.clone())
                    .await?;

                self.transaction_dao.commit(tx).await?;

                // Best-effort file cleanup — warn-log only. The DB is the
                // source of truth; the orphan file (if any) sits at a UUID
                // path and cannot leak via API.
                if let Err(e) = self.document_storage.delete(&old.relative_path).await {
                    tracing::warn!(
                        old_path = %old.relative_path,
                        error = ?e,
                        "Failed to delete old application document file (best-effort)",
                    );
                }

                Ok(ApplicationDocument::from(&updated_entity))
            }
        }
    }

    async fn get(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Option<ApplicationDocument>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // CR-02: check_permission FIRST.
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let existing = self
            .application_document_dao
            .find_active_by_application_id(application_id, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(existing.as_ref().map(ApplicationDocument::from))
    }

    async fn download(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(ApplicationDocument, Vec<u8>), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // CR-02: check_permission FIRST.
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let doc = self
            .application_document_dao
            .find_active_by_application_id(application_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(application_id))?;

        self.transaction_dao.commit(tx).await?;

        let path = doc.relative_path.clone();
        let bytes = self
            .document_storage
            .load(&path)
            .await
            .map_err(|e| match e {
                StorageError::NotFound => ServiceError::InternalError(Arc::from(format!(
                    "Application document file missing on filesystem: {}",
                    path
                ))),
                other => ServiceError::InternalError(Arc::from(other.to_string())),
            })?;

        Ok((ApplicationDocument::from(&doc), bytes))
    }

    async fn delete(
        &self,
        application_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // CR-02: check_permission FIRST.
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let doc = self
            .application_document_dao
            .find_active_by_application_id(application_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(application_id))?;

        let now = now_primitive();
        let new_version = self.uuid_service.new_v4().await;
        let soft_deleted = genossi_dao::application_document::ApplicationDocumentEntity {
            id: doc.id,
            application_id: doc.application_id,
            file_name: doc.file_name.clone(),
            mime_type: doc.mime_type.clone(),
            relative_path: doc.relative_path.clone(),
            size: doc.size,
            created: doc.created,
            deleted: Some(now),
            version: new_version,
        };

        self.application_document_dao
            .update(&soft_deleted, APPLICATION_DOCUMENT_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;

        // Best-effort physical file cleanup.
        if let Err(e) = self.document_storage.delete(&doc.relative_path).await {
            tracing::warn!(
                path = %doc.relative_path,
                error = ?e,
                "Failed to delete application document file (best-effort)",
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::application::{ApplicationDao, ApplicationEntity, ApplicationStatus};
    use genossi_dao::application_document::ApplicationDocumentEntity;
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::{mock, Sequence};

    // ------- Test types -------

    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    // ------- Mocks -------

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(
                &self,
                tx: Option<TestTransaction>,
            ) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestAppDocDao {}
        #[async_trait]
        impl ApplicationDocumentDao for TestAppDocDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &ApplicationDocumentEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &ApplicationDocumentEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<ApplicationDocumentEntity>, DaoError>;
            async fn find_active_by_application_id(
                &self,
                application_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<ApplicationDocumentEntity>, DaoError>;
        }
    }

    mock! {
        pub TestAppDao {}
        #[async_trait]
        impl ApplicationDao for TestAppDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[ApplicationEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &ApplicationEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &ApplicationEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[ApplicationEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<ApplicationEntity>, DaoError>;
        }
    }

    mock! {
        pub TestStorage {}
        #[async_trait]
        impl DocumentStorage for TestStorage {
            async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError>;
            async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
            async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = MockContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<MockContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<MockContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &MockContext) -> Result<bool, ServiceError>;
        }
    }

    #[derive(Clone)]
    struct StaticUuidService;
    #[async_trait]
    impl UuidService for StaticUuidService {
        async fn new_v4(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    struct TestDeps;
    impl ApplicationDocumentServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type ApplicationDocumentDao = MockTestAppDocDao;
        type ApplicationDao = MockTestAppDao;
        type DocumentStorage = MockTestStorage;
        type PermissionService = MockTestPermissionService;
        type UuidService = StaticUuidService;
        type TransactionDao = MockTestTxDao;
    }

    // ------- Helpers -------

    fn make_test_datetime() -> time::PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::July, 3).unwrap();
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
    }

    fn setup_mock_tx_dao() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn permission_admin_ok() -> MockTestPermissionService {
        let mut p = MockTestPermissionService::new();
        p.expect_check_permission().returning(|_, _| Ok(()));
        p.expect_current_user_id()
            .returning(|_| Ok(Some("admin-user".to_string())));
        p
    }

    fn app_entity(id: Uuid) -> ApplicationEntity {
        ApplicationEntity {
            id,
            first_name: Arc::from("Test"),
            last_name: Arc::from("User"),
            salutation: None,
            title: None,
            email: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            shares: 1,
            status: ApplicationStatus::Offen,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn make_existing_doc(app_id: Uuid) -> ApplicationDocumentEntity {
        ApplicationDocumentEntity {
            id: Uuid::new_v4(),
            application_id: app_id,
            file_name: Arc::from("old.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("applications/foo/old.pdf"),
            size: 100,
            created: make_test_datetime(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn build_service(
        app_doc_dao: MockTestAppDocDao,
        app_dao: MockTestAppDao,
        storage: MockTestStorage,
        perm: MockTestPermissionService,
    ) -> ApplicationDocumentServiceImpl<TestDeps> {
        ApplicationDocumentServiceImpl {
            application_document_dao: Arc::new(app_doc_dao),
            application_dao: Arc::new(app_dao),
            document_storage: Arc::new(storage),
            permission_service: Arc::new(perm),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    // ------- Tests -------

    /// Test 1: Happy-path create-new — no existing row, upload calls
    /// `create()` on DAO and `save()` on storage exactly once each.
    #[tokio::test]
    async fn test_upload_create_new_calls_create_then_save() {
        let app_id = Uuid::new_v4();

        let mut app_doc_dao = MockTestAppDocDao::new();
        app_doc_dao
            .expect_find_active_by_application_id()
            .returning(|_, _| Ok(None));
        app_doc_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));
        // Update MUST NOT be called on create-new.
        app_doc_dao.expect_update().times(0);

        let mut app_dao = MockTestAppDao::new();
        app_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(app_entity(app_id))));

        let mut storage = MockTestStorage::new();
        storage
            .expect_save()
            .times(1)
            .withf(|path, _data| path.starts_with("applications/"))
            .returning(|_, _| Ok(()));
        // No old file to delete.
        storage.expect_delete().times(0);

        let service = build_service(app_doc_dao, app_dao, storage, permission_admin_ok());
        let result = service
            .upload(
                UploadApplicationDocument {
                    application_id: app_id,
                    file_name: "antrag.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    data: b"pdf-bytes".to_vec(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await;

        let doc = result.expect("upload create-new must succeed");
        assert_eq!(doc.application_id, app_id);
        assert!(doc
            .relative_path
            .starts_with(&format!("applications/{}/", app_id)));
        assert!(doc.relative_path.ends_with(".pdf"));
    }

    /// Test 2: Replace-in-place — existing row triggers save-new → update →
    /// delete-old sequence. Sequence-enforced via `mockall::Sequence`.
    #[tokio::test]
    async fn test_upload_replace_in_place_calls_save_then_update_then_delete() {
        let app_id = Uuid::new_v4();
        let existing = make_existing_doc(app_id);
        let old_path = existing.relative_path.clone();

        let mut seq = Sequence::new();

        let mut app_doc_dao = MockTestAppDocDao::new();
        {
            let existing_clone = existing.clone();
            app_doc_dao
                .expect_find_active_by_application_id()
                .returning(move |_, _| Ok(Some(existing_clone.clone())));
        }

        let mut storage = MockTestStorage::new();
        storage
            .expect_save()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));

        app_doc_dao
            .expect_update()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _, _| Ok(()));
        // create must NOT be called on replace path.
        app_doc_dao.expect_create().times(0);

        {
            let expected_old: String = old_path.to_string();
            storage
                .expect_delete()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |p| p == expected_old)
                .returning(|_| Ok(()));
        }

        let mut app_dao = MockTestAppDao::new();
        app_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(app_entity(app_id))));

        let service = build_service(app_doc_dao, app_dao, storage, permission_admin_ok());
        let doc = service
            .upload(
                UploadApplicationDocument {
                    application_id: app_id,
                    file_name: "new.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    data: b"new-bytes".to_vec(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect("replace-in-place upload succeeds");

        assert_eq!(doc.id, existing.id, "id must be preserved on replace");
        assert_eq!(doc.file_name.as_ref(), "new.pdf");
        assert!(doc
            .relative_path
            .starts_with(&format!("applications/{}/", app_id)));
    }

    /// Test 3 (CR-02 REGRESSION GUARD, APDOC-02): unauthorised call returns
    /// `PermissionDenied` WITHOUT touching DAO, storage, or capturing
    /// user id. If this test ever fails, the ordering has regressed.
    #[tokio::test]
    async fn test_upload_permission_denied_has_no_side_effects() {
        let app_id = Uuid::new_v4();

        let mut app_doc_dao = MockTestAppDocDao::new();
        app_doc_dao.expect_create().times(0);
        app_doc_dao.expect_update().times(0);
        app_doc_dao.expect_find_active_by_application_id().times(0);

        let mut app_dao = MockTestAppDao::new();
        app_dao.expect_find_by_id().times(0);

        let mut storage = MockTestStorage::new();
        storage.expect_save().times(0);
        storage.expect_delete().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        // current_user_id must NOT be called before check_permission fails
        // (CR-02 ordering: check_permission comes FIRST).
        perm.expect_current_user_id().times(0);

        let service = build_service(app_doc_dao, app_dao, storage, perm);
        let err = service
            .upload(
                UploadApplicationDocument {
                    application_id: app_id,
                    file_name: "x.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    data: b"x".to_vec(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect_err("unauthorised upload must fail");

        assert!(matches!(err, ServiceError::PermissionDenied));
    }

    /// Test 4: download returns InternalError (not EntityNotFound) when the
    /// DB row exists but the file has vanished — signals corruption.
    #[tokio::test]
    async fn test_download_missing_file_returns_internal_error() {
        let app_id = Uuid::new_v4();
        let existing = make_existing_doc(app_id);

        let mut app_doc_dao = MockTestAppDocDao::new();
        {
            let existing_clone = existing.clone();
            app_doc_dao
                .expect_find_active_by_application_id()
                .returning(move |_, _| Ok(Some(existing_clone.clone())));
        }

        let mut storage = MockTestStorage::new();
        storage
            .expect_load()
            .returning(|_| Err(StorageError::NotFound));

        let app_dao = MockTestAppDao::new();

        let service = build_service(app_doc_dao, app_dao, storage, permission_admin_ok());
        let err = service
            .download(app_id, Authentication::Context(MockContext), None)
            .await
            .expect_err("missing file → error");

        match err {
            ServiceError::InternalError(msg) => {
                assert!(
                    msg.contains("missing on filesystem"),
                    "message must mention filesystem corruption, got: {}",
                    msg
                );
            }
            other => panic!("expected InternalError, got {:?}", other),
        }
    }

    /// Test 5: delete swallows storage.delete errors as warn-only. Returns
    /// `Ok(())` even when the file cannot be removed; DAO update was called.
    #[tokio::test]
    async fn test_delete_storage_failure_is_swallowed() {
        let app_id = Uuid::new_v4();
        let existing = make_existing_doc(app_id);

        let mut app_doc_dao = MockTestAppDocDao::new();
        {
            let existing_clone = existing.clone();
            app_doc_dao
                .expect_find_active_by_application_id()
                .returning(move |_, _| Ok(Some(existing_clone.clone())));
        }
        app_doc_dao
            .expect_update()
            .times(1)
            .withf(
                |entity: &ApplicationDocumentEntity, _proc: &str, _tx: &TestTransaction| {
                    entity.deleted.is_some()
                },
            )
            .returning(|_, _, _| Ok(()));

        let mut storage = MockTestStorage::new();
        storage
            .expect_delete()
            .returning(|_| Err(StorageError::NotFound));

        let app_dao = MockTestAppDao::new();

        let service = build_service(app_doc_dao, app_dao, storage, permission_admin_ok());
        let result = service
            .delete(app_id, Authentication::Context(MockContext), None)
            .await;

        assert!(
            result.is_ok(),
            "delete must swallow storage.delete errors, got {:?}",
            result
        );
    }

    /// Test 6: get returns `Ok(None)` when no active row exists — the
    /// "no document yet" state is normal.
    #[tokio::test]
    async fn test_get_returns_none_when_no_active_row() {
        let app_id = Uuid::new_v4();

        let mut app_doc_dao = MockTestAppDocDao::new();
        app_doc_dao
            .expect_find_active_by_application_id()
            .returning(|_, _| Ok(None));

        let app_dao = MockTestAppDao::new();
        let storage = MockTestStorage::new();

        let service = build_service(app_doc_dao, app_dao, storage, permission_admin_ok());
        let result = service
            .get(app_id, Authentication::Context(MockContext), None)
            .await
            .expect("get must succeed");

        assert!(result.is_none(), "no active row → Ok(None), not error");
    }

    // ------- Additional helper coverage -------

    #[test]
    fn test_extract_extension_basic() {
        assert_eq!(extract_extension("a.pdf"), Some("pdf".into()));
        assert_eq!(extract_extension("noext"), None);
        assert_eq!(extract_extension("weird.EXE"), Some("exe".into()));
    }
}
