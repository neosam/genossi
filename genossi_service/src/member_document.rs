use async_trait::async_trait;
use genossi_dao::member_document::MemberDocumentEntity;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentType {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
}

impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            DocumentType::JoinDeclaration => "join_declaration",
            DocumentType::JoinConfirmation => "join_confirmation",
            DocumentType::ShareIncrease => "share_increase",
            DocumentType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_declaration" => Some(DocumentType::JoinDeclaration),
            "join_confirmation" => Some(DocumentType::JoinConfirmation),
            "share_increase" => Some(DocumentType::ShareIncrease),
            "other" => Some(DocumentType::Other),
            _ => None,
        }
    }

    pub fn is_singleton(&self) -> bool {
        matches!(self, DocumentType::JoinDeclaration | DocumentType::JoinConfirmation)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberDocument {
    pub id: Uuid,
    pub member_id: Uuid,
    pub document_type: DocumentType,
    pub description: Option<Arc<str>>,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&MemberDocumentEntity> for MemberDocument {
    fn from(entity: &MemberDocumentEntity) -> Self {
        Self {
            id: entity.id,
            member_id: entity.member_id,
            document_type: DocumentType::from_str(&entity.document_type)
                .unwrap_or(DocumentType::Other),
            description: entity.description.clone(),
            file_name: entity.file_name.clone(),
            mime_type: entity.mime_type.clone(),
            relative_path: entity.relative_path.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&MemberDocument> for MemberDocumentEntity {
    fn from(doc: &MemberDocument) -> Self {
        Self {
            id: doc.id,
            member_id: doc.member_id,
            document_type: Arc::from(doc.document_type.as_str()),
            description: doc.description.clone(),
            file_name: doc.file_name.clone(),
            mime_type: doc.mime_type.clone(),
            relative_path: doc.relative_path.clone(),
            created: doc.created,
            deleted: doc.deleted,
            version: doc.version,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UploadDocument {
    pub member_id: Uuid,
    pub document_type: DocumentType,
    pub description: Option<String>,
    pub file_name: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MemberDocumentService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn upload(
        &self,
        upload: UploadDocument,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<MemberDocument, ServiceError>;

    async fn list(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[MemberDocument]>, ServiceError>;

    async fn download(
        &self,
        member_id: Uuid,
        document_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(MemberDocument, Vec<u8>), ServiceError>;

    async fn delete(
        &self,
        member_id: Uuid,
        document_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}
