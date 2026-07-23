//! Impl for [`MailAssetService`] (Phase 27, IMG-01/02/04/05).
//!
//! CR-02 (STRICTER than the application_document analog): `check_permission`
//! is the FIRST statement of every method — before `use_transaction`, before
//! any DAO call, before capturing the user id. The analog opens the
//! transaction first; we deliberately do NOT copy that ordering because the
//! project CR-02 invariant and the regression-guard test require zero side
//! effects on an authorisation denial.
//!
//! Storage: bytes live INLINE in the entity's `bytes: Vec<u8>` BLOB column —
//! no document-storage dependency, no filesystem persistence.
//!
//! MIME validation: a magic-byte sniff (`sniff_image_mime`) inspects the
//! payload prefix and accepts ONLY PNG/JPEG/GIF. The client-declared MIME and
//! filename extension are ignored (IMG-05: both are spoofable). The
//! server-derived MIME is what gets stored.

use async_trait::async_trait;
use genossi_dao::mail_asset::MailAssetDao;
use genossi_dao::TransactionDao;
use genossi_service::mail_asset::{MailAsset, MailAssetService, UploadMailAsset};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

/// Process tag stored in DAO write calls (required by the trait even though
/// this entity is NOT audited).
const MAIL_ASSET_PROCESS: &str = "mail-asset-service";
const ADMIN_PRIVILEGE: &str = "admin";
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5 MB per image (IMG-02).

gen_service_impl! {
    struct MailAssetServiceImpl: MailAssetService = MailAssetServiceDeps {
        MailAssetDao: MailAssetDao<Transaction = Self::Transaction> = mail_asset_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

/// Magic-byte MIME sniff (IMG-05 / Pitfall 4). Inspects the payload prefix and
/// returns the canonical MIME for PNG/JPEG/GIF, or `None` for anything else
/// (SVG, polyglots, non-images). Client Content-Type and filename extension
/// are never consulted.
fn sniff_image_mime(data: &[u8]) -> Option<&'static str> {
    // PNG: 89 50 4E 47 0D 0A 1A 0A — we check the leading 6 bytes (\x89PNG\r\n).
    const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
    // JPEG: FF D8 FF
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF];
    // GIF: "GIF87a" / "GIF89a"
    const GIF87A: &[u8] = b"GIF87a";
    const GIF89A: &[u8] = b"GIF89a";

    if data.starts_with(PNG) {
        Some("image/png")
    } else if data.starts_with(JPEG) {
        Some("image/jpeg")
    } else if data.starts_with(GIF87A) || data.starts_with(GIF89A) {
        Some("image/gif")
    } else {
        None
    }
}

fn now_primitive() -> time::PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(now.date(), now.time())
}

#[async_trait]
impl<Deps: MailAssetServiceDeps> MailAssetService for MailAssetServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn upload(
        &self,
        upload: UploadMailAsset,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MailAsset, ServiceError> {
        // CR-02: check_permission FIRST — before use_transaction, before any
        // DAO call, before capturing user_id. No side effects on denial.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        // Validate file size (defense-in-depth alongside the REST body limit).
        if upload.data.len() > MAX_FILE_SIZE {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("file"),
                message: Arc::from("File size exceeds 5 MB limit"),
            }]));
        }

        // Magic-byte MIME sniff — reject non-PNG/JPEG/GIF payloads (SVG,
        // polyglots). The REST layer maps this validation error to 415.
        let sniffed = sniff_image_mime(&upload.data).ok_or_else(|| {
            ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("file"),
                message: Arc::from(
                    "Unsupported image type — only PNG, JPEG and GIF are allowed",
                ),
            }])
        })?;

        let tx = self.transaction_dao.use_transaction(tx).await?;

        let entity = genossi_dao::mail_asset::MailAssetEntity {
            id: self.uuid_service.new_v4().await,
            filename: Arc::from(upload.filename.as_str()),
            mime_type: Arc::from(sniffed),
            size_bytes: upload.data.len() as i64,
            bytes: upload.data,
            uploaded_by: Arc::from(user_id.as_str()),
            created: now_primitive(),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.mail_asset_dao
            .create(&entity, MAIL_ASSET_PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;

        Ok(MailAsset::from(&entity))
    }

    async fn download(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MailAsset, Vec<u8>), ServiceError> {
        // CR-02: check_permission FIRST.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let tx = self.transaction_dao.use_transaction(tx).await?;

        let entity = self
            .mail_asset_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;

        let bytes = entity.bytes.clone();
        Ok((MailAsset::from(&entity), bytes))
    }

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MailAsset, ServiceError> {
        // CR-02: check_permission FIRST.
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        let _user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let tx = self.transaction_dao.use_transaction(tx).await?;

        let entity = self
            .mail_asset_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;

        Ok(MailAsset::from(&entity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::mail_asset::MailAssetEntity;
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;

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
        pub TestMailAssetDao {}
        #[async_trait]
        impl MailAssetDao for TestMailAssetDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MailAssetEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &MailAssetEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &MailAssetEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MailAssetEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<MailAssetEntity>, DaoError>;
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
    impl MailAssetServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTransaction;
        type MailAssetDao = MockTestMailAssetDao;
        type PermissionService = MockTestPermissionService;
        type UuidService = StaticUuidService;
        type TransactionDao = MockTestTxDao;
    }

    // ------- Helpers -------

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

    fn build_service(
        dao: MockTestMailAssetDao,
        perm: MockTestPermissionService,
    ) -> MailAssetServiceImpl<TestDeps> {
        MailAssetServiceImpl {
            mail_asset_dao: Arc::new(dao),
            permission_service: Arc::new(perm),
            uuid_service: Arc::new(StaticUuidService),
            transaction_dao: Arc::new(setup_mock_tx_dao()),
        }
    }

    fn png_bytes() -> Vec<u8> {
        // \x89PNG\r\n + trailing bytes.
        vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01]
    }

    // ------- sniff unit tests -------

    #[test]
    fn test_sniff_accepts_png_jpeg_gif() {
        assert_eq!(sniff_image_mime(&png_bytes()), Some("image/png"));
        assert_eq!(sniff_image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff_image_mime(b"GIF87a...."), Some("image/gif"));
        assert_eq!(sniff_image_mime(b"GIF89a...."), Some("image/gif"));
    }

    #[test]
    fn test_sniff_rejects_svg_and_non_images() {
        assert_eq!(sniff_image_mime(b"<svg xmlns=\"...\">"), None);
        assert_eq!(sniff_image_mime(b"<?xml version=\"1.0\"?>"), None);
        assert_eq!(sniff_image_mime(b"%PDF-1.4"), None);
        assert_eq!(sniff_image_mime(b""), None);
    }

    // ------- service tests -------

    /// Happy path: valid PNG → create called once, server-derived mime is
    /// "image/png" (client mime ignored).
    #[tokio::test]
    async fn test_upload_valid_png_calls_create_once_with_server_mime() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_create()
            .times(1)
            .withf(|entity: &MailAssetEntity, _proc: &str, _tx: &TestTransaction| {
                entity.mime_type.as_ref() == "image/png"
            })
            .returning(|_, _, _| Ok(()));

        let service = build_service(dao, permission_admin_ok());
        let asset = service
            .upload(
                UploadMailAsset {
                    filename: "logo.png".to_string(),
                    // Client-declared MIME is a lie; must be ignored.
                    mime_type: "image/gif".to_string(),
                    data: png_bytes(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect("valid PNG upload must succeed");

        assert_eq!(asset.mime_type.as_ref(), "image/png");
        assert_eq!(asset.filename.as_ref(), "logo.png");
        assert_eq!(asset.uploaded_by.as_ref(), "admin-user");
        assert_eq!(asset.size_bytes, png_bytes().len() as i64);
    }

    /// CR-02 REGRESSION GUARD: unauthorised upload returns PermissionDenied
    /// WITHOUT any DAO call and WITHOUT capturing user_id (check_permission is
    /// the FIRST statement).
    #[tokio::test]
    async fn test_upload_permission_denied_has_no_side_effects() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_create().times(0);
        dao.expect_update().times(0);
        dao.expect_find_by_id().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        // current_user_id must NOT be called before check_permission fails.
        perm.expect_current_user_id().times(0);

        let service = build_service(dao, perm);
        let err = service
            .upload(
                UploadMailAsset {
                    filename: "logo.png".to_string(),
                    mime_type: "image/png".to_string(),
                    data: png_bytes(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect_err("unauthorised upload must fail");

        assert!(matches!(err, ServiceError::PermissionDenied));
    }

    /// Payload > 5 MB → ValidationError with field "file"; zero DAO calls.
    #[tokio::test]
    async fn test_upload_oversized_returns_validation_error() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_create().times(0);

        // Build a >5 MB payload that starts with a valid PNG prefix (so size,
        // not MIME, is the rejection reason).
        let mut data = png_bytes();
        data.resize(MAX_FILE_SIZE + 1, 0x00);

        let service = build_service(dao, permission_admin_ok());
        let err = service
            .upload(
                UploadMailAsset {
                    filename: "big.png".to_string(),
                    mime_type: "image/png".to_string(),
                    data,
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect_err("oversized upload must fail");

        match err {
            ServiceError::ValidationError(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].field.as_ref(), "file");
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }
    }

    /// SVG / non-image payload → rejected; zero DAO create calls.
    #[tokio::test]
    async fn test_upload_svg_rejected_no_dao_call() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_create().times(0);

        let service = build_service(dao, permission_admin_ok());
        let err = service
            .upload(
                UploadMailAsset {
                    // A .png filename + image/png client MIME must NOT rescue
                    // SVG bytes — the sniff wins.
                    filename: "sneaky.png".to_string(),
                    mime_type: "image/png".to_string(),
                    data: b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec(),
                },
                Authentication::Context(MockContext),
                None,
            )
            .await
            .expect_err("SVG upload must fail");

        assert!(matches!(err, ServiceError::ValidationError(_)));
    }

    /// download by admin returns the stored bytes.
    #[tokio::test]
    async fn test_download_admin_returns_bytes() {
        let entity = MailAssetEntity {
            id: Uuid::new_v4(),
            filename: Arc::from("logo.png"),
            mime_type: Arc::from("image/png"),
            size_bytes: png_bytes().len() as i64,
            bytes: png_bytes(),
            uploaded_by: Arc::from("admin-user"),
            created: now_primitive(),
            deleted: None,
            version: Uuid::new_v4(),
        };
        let entity_id = entity.id;
        let expected = entity.bytes.clone();

        let mut dao = MockTestMailAssetDao::new();
        dao.expect_find_by_id()
            .returning(move |_, _| Ok(Some(entity.clone())));

        let service = build_service(dao, permission_admin_ok());
        let (asset, bytes) = service
            .download(entity_id, Authentication::Context(MockContext), None)
            .await
            .expect("admin download must succeed");

        assert_eq!(asset.id, entity_id);
        assert_eq!(bytes, expected);
    }

    /// download by non-admin → PermissionDenied, zero DAO calls.
    #[tokio::test]
    async fn test_download_non_admin_denied() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_find_by_id().times(0);

        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        perm.expect_current_user_id().times(0);

        let service = build_service(dao, perm);
        let err = service
            .download(Uuid::new_v4(), Authentication::Context(MockContext), None)
            .await
            .expect_err("non-admin download must fail");

        assert!(matches!(err, ServiceError::PermissionDenied));
    }

    /// download of a missing/soft-deleted asset → EntityNotFound.
    #[tokio::test]
    async fn test_download_missing_returns_not_found() {
        let mut dao = MockTestMailAssetDao::new();
        dao.expect_find_by_id().returning(|_, _| Ok(None));

        let service = build_service(dao, permission_admin_ok());
        let err = service
            .download(Uuid::new_v4(), Authentication::Context(MockContext), None)
            .await
            .expect_err("missing asset must fail");

        assert!(matches!(err, ServiceError::EntityNotFound(_)));
    }
}
