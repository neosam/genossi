use async_trait::async_trait;
use genossi_dao::member_document::MemberDocumentEntity;
use mockall::automock;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

pub const ALLOWED_FILE_TYPES: &[(&str, &str)] = &[
    ("pdf", "application/pdf"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("webp", "image/webp"),
    ("txt", "text/plain"),
    ("doc", "application/msword"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ),
    ("odt", "application/vnd.oasis.opendocument.text"),
    ("xls", "application/vnd.ms-excel"),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ),
    ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
];

/// Look up the MIME type for a given extension (case-insensitive).
/// Returns `None` if the extension is not in the whitelist.
pub fn lookup_allowed_mime(extension: &str) -> Option<&'static str> {
    let lower = extension.to_ascii_lowercase();
    ALLOWED_FILE_TYPES
        .iter()
        .find(|(ext, _)| *ext == lower)
        .map(|(_, mime)| *mime)
}

/// Returns the list of allowed file extensions.
pub fn allowed_extensions() -> Vec<&'static str> {
    ALLOWED_FILE_TYPES.iter().map(|(ext, _)| *ext).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentType {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
    // Phase 10 D-09: persistent anchor for repayment-mail send events.
    // Non-singleton (multiple mails per member allowed); no Typst template
    // (the mail body itself is the artifact, no PDF generation).
    RepaymentMail,
}

impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            DocumentType::JoinDeclaration => "join_declaration",
            DocumentType::JoinConfirmation => "join_confirmation",
            DocumentType::ShareIncrease => "share_increase",
            DocumentType::Other => "other",
            DocumentType::RepaymentMail => "repayment_mail",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_declaration" => Some(DocumentType::JoinDeclaration),
            "join_confirmation" => Some(DocumentType::JoinConfirmation),
            "share_increase" => Some(DocumentType::ShareIncrease),
            "other" => Some(DocumentType::Other),
            "repayment_mail" => Some(DocumentType::RepaymentMail),
            _ => None,
        }
    }

    pub fn is_singleton(&self) -> bool {
        matches!(
            self,
            DocumentType::JoinDeclaration | DocumentType::JoinConfirmation
        )
    }

    /// Returns the Typst template path for document types that support generation.
    /// Returns `None` for types without a template mapping (e.g. `Other`, `ShareIncrease`,
    /// `RepaymentMail`).
    pub fn template_path(&self) -> Option<&str> {
        match self {
            DocumentType::JoinConfirmation => Some("join_confirmation.typ"),
            DocumentType::JoinDeclaration => Some("join_declaration.typ"),
            DocumentType::ShareIncrease => None,
            DocumentType::Other => None,
            DocumentType::RepaymentMail => None,
        }
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
    // Phase 10 D-07 (MAIL-03/04): optional mail-tracking fields mirrored from
    // MemberDocumentEntity so service-layer code can construct full audited
    // documents without falling back to the DAO entity directly.
    pub template_id: Option<Uuid>,
    pub mail_recipient_id: Option<Uuid>,
    pub status: Option<Arc<str>>,
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
            template_id: entity.template_id,
            mail_recipient_id: entity.mail_recipient_id,
            status: entity.status.clone(),
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
            template_id: doc.template_id,
            mail_recipient_id: doc.mail_recipient_id,
            status: doc.status.clone(),
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

    async fn count_by_type(
        &self,
        document_type: DocumentType,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<HashMap<Uuid, i64>, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_allowed_mime_pdf() {
        assert_eq!(lookup_allowed_mime("pdf"), Some("application/pdf"));
    }

    #[test]
    fn test_lookup_allowed_mime_case_insensitive() {
        assert_eq!(lookup_allowed_mime("PDF"), Some("application/pdf"));
        assert_eq!(lookup_allowed_mime("Pdf"), Some("application/pdf"));
        assert_eq!(lookup_allowed_mime("JPG"), Some("image/jpeg"));
    }

    #[test]
    fn test_lookup_allowed_mime_not_whitelisted() {
        assert_eq!(lookup_allowed_mime("exe"), None);
        assert_eq!(lookup_allowed_mime("html"), None);
        assert_eq!(lookup_allowed_mime("zip"), None);
        assert_eq!(lookup_allowed_mime("gz"), None);
    }

    #[test]
    fn test_allowed_extensions_contains_all() {
        let exts = allowed_extensions();
        assert!(exts.contains(&"pdf"));
        assert!(exts.contains(&"png"));
        assert!(exts.contains(&"jpg"));
        assert!(exts.contains(&"jpeg"));
        assert!(exts.contains(&"docx"));
        assert!(exts.contains(&"odt"));
        assert!(exts.contains(&"ods"));
        assert_eq!(exts.len(), ALLOWED_FILE_TYPES.len());
    }

    // -------------------------------------------------------------------------
    // Phase 10 D-09: DocumentType::RepaymentMail variant tests
    // RepaymentMail anchors a sent/failed mail event; non-singleton (multi-mail
    // per member allowed), no Typst template (mail body, not document).
    // -------------------------------------------------------------------------

    #[test]
    fn test_document_type_repayment_mail_as_str() {
        assert_eq!(DocumentType::RepaymentMail.as_str(), "repayment_mail");
    }

    #[test]
    fn test_document_type_repayment_mail_from_str() {
        assert_eq!(
            DocumentType::from_str("repayment_mail"),
            Some(DocumentType::RepaymentMail)
        );
    }

    #[test]
    fn test_document_type_repayment_mail_is_not_singleton() {
        assert!(
            !DocumentType::RepaymentMail.is_singleton(),
            "RepaymentMail must allow multiple mails per member (CONTEXT D-09)"
        );
    }

    #[test]
    fn test_document_type_repayment_mail_no_template_path() {
        assert_eq!(
            DocumentType::RepaymentMail.template_path(),
            None,
            "RepaymentMail has no Typst template (CONTEXT D-09)"
        );
    }
}
