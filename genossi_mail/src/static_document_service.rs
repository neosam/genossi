use async_trait::async_trait;
use genossi_service::document_storage::DocumentStorage;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{MailDaoError, StaticDocument, StaticDocumentDao};

#[derive(Debug, Clone)]
pub enum StaticDocumentError {
    DataAccess(Arc<str>),
    Storage(Arc<str>),
    NotFound,
    Validation(Arc<str>),
}

impl From<MailDaoError> for StaticDocumentError {
    fn from(e: MailDaoError) -> Self {
        match e {
            MailDaoError::DatabaseError(msg) => StaticDocumentError::DataAccess(msg),
            MailDaoError::NotFound => StaticDocumentError::NotFound,
        }
    }
}

pub const DEFAULT_MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
pub const ALLOWED_CONTENT_TYPES: &[&str] = &["application/pdf", "image/png", "image/jpeg"];

pub struct UploadStaticDocument {
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[async_trait]
pub trait StaticDocumentService: Send + Sync + 'static {
    async fn upload(
        &self,
        upload: UploadStaticDocument,
    ) -> Result<StaticDocument, StaticDocumentError>;
    async fn list(&self) -> Result<Arc<[StaticDocument]>, StaticDocumentError>;
    async fn find_by_id(&self, id: Uuid) -> Result<StaticDocument, StaticDocumentError>;
    async fn load_bytes(&self, id: Uuid) -> Result<(StaticDocument, Vec<u8>), StaticDocumentError>;
    async fn delete(&self, id: Uuid) -> Result<(), StaticDocumentError>;
}

pub struct StaticDocumentServiceImpl<D: StaticDocumentDao, S: DocumentStorage> {
    dao: Arc<D>,
    storage: Arc<S>,
    max_size_bytes: u64,
}

impl<D: StaticDocumentDao, S: DocumentStorage> StaticDocumentServiceImpl<D, S> {
    pub fn new(dao: Arc<D>, storage: Arc<S>) -> Self {
        let max_size_bytes = std::env::var("STATIC_DOCUMENTS_MAX_BYTES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_SIZE_BYTES);
        Self {
            dao,
            storage,
            max_size_bytes,
        }
    }
}

#[async_trait]
impl<D: StaticDocumentDao, S: DocumentStorage + 'static> StaticDocumentService
    for StaticDocumentServiceImpl<D, S>
{
    async fn upload(
        &self,
        upload: UploadStaticDocument,
    ) -> Result<StaticDocument, StaticDocumentError> {
        if !ALLOWED_CONTENT_TYPES.contains(&upload.content_type.as_str()) {
            return Err(StaticDocumentError::Validation(Arc::from(format!(
                "Content type '{}' is not allowed",
                upload.content_type
            ))));
        }
        if (upload.data.len() as u64) > self.max_size_bytes {
            return Err(StaticDocumentError::Validation(Arc::from(format!(
                "File size {} exceeds limit of {} bytes",
                upload.data.len(),
                self.max_size_bytes
            ))));
        }
        if upload.name.trim().is_empty() {
            return Err(StaticDocumentError::Validation(Arc::from(
                "Name must not be empty",
            )));
        }

        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());

        let doc = StaticDocument {
            id: Uuid::new_v4(),
            created: now_primitive,
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from(upload.name.as_str()),
            filename: Arc::from(upload.filename.as_str()),
            content_type: Arc::from(upload.content_type.as_str()),
            size_bytes: upload.data.len() as i64,
        };

        // Filesystem first, DB second. On DB failure, attempt to clean up file.
        let rel_path = doc.relative_path();
        self.storage
            .save(&rel_path, &upload.data)
            .await
            .map_err(|e| StaticDocumentError::Storage(Arc::from(e.to_string())))?;

        if let Err(e) = self.dao.create(&doc).await {
            let _ = self.storage.delete(&rel_path).await;
            return Err(e.into());
        }

        Ok(doc)
    }

    async fn list(&self) -> Result<Arc<[StaticDocument]>, StaticDocumentError> {
        Ok(self.dao.all_active().await?)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<StaticDocument, StaticDocumentError> {
        self.dao
            .find_by_id(id)
            .await?
            .ok_or(StaticDocumentError::NotFound)
    }

    async fn load_bytes(&self, id: Uuid) -> Result<(StaticDocument, Vec<u8>), StaticDocumentError> {
        let doc = self.find_by_id(id).await?;
        let data = self
            .storage
            .load(&doc.relative_path())
            .await
            .map_err(|e| StaticDocumentError::Storage(Arc::from(e.to_string())))?;
        Ok((doc, data))
    }

    async fn delete(&self, id: Uuid) -> Result<(), StaticDocumentError> {
        self.dao.soft_delete(id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::MockStaticDocumentDao;
    use genossi_service::document_storage::MockDocumentStorage;

    fn make_service(
        dao: MockStaticDocumentDao,
        storage: MockDocumentStorage,
    ) -> StaticDocumentServiceImpl<MockStaticDocumentDao, MockDocumentStorage> {
        StaticDocumentServiceImpl {
            dao: Arc::new(dao),
            storage: Arc::new(storage),
            max_size_bytes: 1024 * 1024,
        }
    }

    #[tokio::test]
    async fn test_upload_rejects_unsupported_content_type() {
        let service = make_service(MockStaticDocumentDao::new(), MockDocumentStorage::new());
        let result = service
            .upload(UploadStaticDocument {
                name: "X".into(),
                filename: "x.exe".into(),
                content_type: "application/x-msdownload".into(),
                data: vec![0, 1, 2],
            })
            .await;
        assert!(matches!(result, Err(StaticDocumentError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_rejects_oversized_file() {
        let service = make_service(MockStaticDocumentDao::new(), MockDocumentStorage::new());
        let result = service
            .upload(UploadStaticDocument {
                name: "X".into(),
                filename: "x.pdf".into(),
                content_type: "application/pdf".into(),
                data: vec![0u8; 2 * 1024 * 1024],
            })
            .await;
        assert!(matches!(result, Err(StaticDocumentError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_rejects_empty_name() {
        let service = make_service(MockStaticDocumentDao::new(), MockDocumentStorage::new());
        let result = service
            .upload(UploadStaticDocument {
                name: "   ".into(),
                filename: "x.pdf".into(),
                content_type: "application/pdf".into(),
                data: vec![0, 1, 2],
            })
            .await;
        assert!(matches!(result, Err(StaticDocumentError::Validation(_))));
    }

    #[tokio::test]
    async fn test_upload_persists_file_then_db() {
        let mut dao = MockStaticDocumentDao::new();
        let mut storage = MockDocumentStorage::new();
        storage.expect_save().times(1).returning(|_, _| Ok(()));
        dao.expect_create().times(1).returning(|_| Ok(()));

        let service = make_service(dao, storage);
        let result = service
            .upload(UploadStaticDocument {
                name: "Satzung".into(),
                filename: "satzung.pdf".into(),
                content_type: "application/pdf".into(),
                data: vec![1, 2, 3],
            })
            .await
            .unwrap();
        assert_eq!(result.name.as_ref(), "Satzung");
        assert_eq!(result.size_bytes, 3);
    }

    #[tokio::test]
    async fn test_upload_rolls_back_file_on_db_failure() {
        let mut dao = MockStaticDocumentDao::new();
        let mut storage = MockDocumentStorage::new();
        storage.expect_save().times(1).returning(|_, _| Ok(()));
        storage.expect_delete().times(1).returning(|_| Ok(()));
        dao.expect_create()
            .times(1)
            .returning(|_| Err(MailDaoError::DatabaseError(Arc::from("boom"))));

        let service = make_service(dao, storage);
        let result = service
            .upload(UploadStaticDocument {
                name: "Satzung".into(),
                filename: "satzung.pdf".into(),
                content_type: "application/pdf".into(),
                data: vec![1, 2, 3],
            })
            .await;
        assert!(matches!(result, Err(StaticDocumentError::DataAccess(_))));
    }
}
