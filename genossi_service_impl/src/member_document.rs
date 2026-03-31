use async_trait::async_trait;
use genossi_dao::member::MemberDao;
use genossi_dao::member_document::MemberDocumentDao;
use genossi_dao::TransactionDao;
use genossi_service::member_document::{
    DocumentType, MemberDocument, MemberDocumentService, UploadDocument,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const PROCESS: &str = "member-document-service";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB

gen_service_impl! {
    struct MemberDocumentServiceImpl: MemberDocumentService = MemberDocumentServiceDeps {
        MemberDocumentDao: MemberDocumentDao<Transaction = Self::Transaction> = member_document_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

impl<Deps: MemberDocumentServiceDeps> MemberDocumentServiceImpl<Deps> {
    fn extract_extension(file_name: &str) -> &str {
        file_name
            .rsplit('.')
            .next()
            .filter(|ext| ext.len() <= 10 && *ext != file_name)
            .unwrap_or("bin")
    }
}

#[async_trait]
impl<Deps: MemberDocumentServiceDeps> MemberDocumentService
    for MemberDocumentServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn upload(
        &self,
        upload: UploadDocument,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberDocument, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        // Validate file size
        if upload.data.len() > MAX_FILE_SIZE {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("file"),
                message: Arc::from("File size exceeds 50 MB limit"),
            }]));
        }

        // Validate Other requires description
        if upload.document_type == DocumentType::Other && upload.description.as_ref().map_or(true, |d| d.trim().is_empty()) {
            return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
                field: Arc::from("description"),
                message: Arc::from("Description is required for document type 'Other'"),
            }]));
        }

        // Verify member exists
        self.member_dao
            .find_by_id(upload.member_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(upload.member_id))?;

        // Singleton replacement: soft-delete existing document of same type
        if upload.document_type.is_singleton() {
            let existing_docs = self
                .member_document_dao
                .find_by_member_id(upload.member_id, tx.clone())
                .await?;
            for doc in existing_docs.iter() {
                if doc.document_type.as_ref() == upload.document_type.as_str() {
                    let mut to_delete = doc.clone();
                    let now = time::OffsetDateTime::now_utc();
                    to_delete.deleted =
                        Some(time::PrimitiveDateTime::new(now.date(), now.time()));
                    self.member_document_dao
                        .update(&to_delete, PROCESS, tx.clone())
                        .await?;
                }
            }
        }

        let doc_id = self.uuid_service.new_v4().await;
        let extension = Self::extract_extension(&upload.file_name);
        let relative_path = format!("{}.{}", doc_id, extension);

        let now = time::OffsetDateTime::now_utc();
        let new_doc = MemberDocument {
            id: doc_id,
            member_id: upload.member_id,
            document_type: upload.document_type,
            description: upload.description.map(|d| Arc::from(d.as_str())),
            file_name: Arc::from(upload.file_name.as_str()),
            mime_type: Arc::from(upload.mime_type.as_str()),
            relative_path: Arc::from(relative_path.as_str()),
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.member_document_dao
            .create(&(&new_doc).into(), PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(new_doc)
    }

    async fn list(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[MemberDocument]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let docs = self
            .member_document_dao
            .find_by_member_id(member_id, tx.clone())
            .await?;

        let result: Arc<[MemberDocument]> = docs.iter().map(MemberDocument::from).collect();

        self.transaction_dao.commit(tx).await?;
        Ok(result)
    }

    async fn download(
        &self,
        member_id: Uuid,
        document_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberDocument, Vec<u8>), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let doc = self
            .member_document_dao
            .find_by_id(document_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(document_id))?;

        if doc.member_id != member_id {
            return Err(ServiceError::EntityNotFound(document_id));
        }

        self.transaction_dao.commit(tx).await?;

        let doc = MemberDocument::from(&doc);
        Ok((doc, Vec::new())) // Bytes loaded by REST layer from storage
    }

    async fn delete(
        &self,
        member_id: Uuid,
        document_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let doc = self
            .member_document_dao
            .find_by_id(document_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(document_id))?;

        if doc.member_id != member_id {
            return Err(ServiceError::EntityNotFound(document_id));
        }

        let mut to_delete = doc.clone();
        let now = time::OffsetDateTime::now_utc();
        to_delete.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));

        self.member_document_dao
            .update(&to_delete, PROCESS, tx.clone())
            .await?;

        self.transaction_dao.commit(tx).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_extension uses the same logic inline for testability
    fn extract_extension_logic(file_name: &str) -> &str {
        file_name
            .rsplit('.')
            .next()
            .filter(|ext| ext.len() <= 10 && *ext != file_name)
            .unwrap_or("bin")
    }

    #[test]
    fn test_extract_extension() {
        assert_eq!(extract_extension_logic("test.pdf"), "pdf");
        assert_eq!(extract_extension_logic("photo.jpg"), "jpg");
        assert_eq!(extract_extension_logic("noext"), "bin");
        assert_eq!(extract_extension_logic("archive.tar.gz"), "gz");
    }

    #[test]
    fn test_document_type_singleton() {
        assert!(DocumentType::JoinDeclaration.is_singleton());
        assert!(DocumentType::JoinConfirmation.is_singleton());
        assert!(!DocumentType::ShareIncrease.is_singleton());
        assert!(!DocumentType::Other.is_singleton());
    }

    #[test]
    fn test_document_type_roundtrip() {
        for dt in [
            DocumentType::JoinDeclaration,
            DocumentType::JoinConfirmation,
            DocumentType::ShareIncrease,
            DocumentType::Other,
        ] {
            let s = dt.as_str();
            let parsed = DocumentType::from_str(s).unwrap();
            assert_eq!(parsed, dt);
        }
    }

    #[test]
    fn test_document_type_from_invalid() {
        assert!(DocumentType::from_str("invalid").is_none());
    }
}
