use async_trait::async_trait;
use mockall::automock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberDocumentEntity {
    pub id: Uuid,
    pub member_id: Uuid,
    pub document_type: Arc<str>,
    pub description: Option<Arc<str>>,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
    // Phase 10 D-07 (MAIL-03/04): optional fields for repayment-mail tracking.
    // Legacy MemberDocuments (JoinDeclaration etc.) keep NULL in these columns.
    pub template_id: Option<Uuid>,
    pub mail_recipient_id: Option<Uuid>,
    pub status: Option<Arc<str>>,
}

impl crate::auditable::Auditable for MemberDocumentEntity {
    fn entity_type() -> &'static str {
        "member_document"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        // FROZEN ORDER (Hash-Chain-Konsistenz, Phase-7-Lektion):
        // Existing fields stay at indices 0-5 — modifying their order would break
        // historical audit replay. Phase-10 fields appended at indices 6-8.
        // member_id, document_type, description, file_name, mime_type, relative_path,
        // template_id, mail_recipient_id, status
        vec![
            ("member_id", Some(self.member_id.to_string())),
            ("document_type", Some(self.document_type.to_string())),
            (
                "description",
                self.description.as_ref().map(|s| s.to_string()),
            ),
            ("file_name", Some(self.file_name.to_string())),
            ("mime_type", Some(self.mime_type.to_string())),
            ("relative_path", Some(self.relative_path.to_string())),
            (
                "template_id",
                self.template_id.as_ref().map(|u| u.to_string()),
            ),
            (
                "mail_recipient_id",
                self.mail_recipient_id.as_ref().map(|u| u.to_string()),
            ),
            ("status", self.status.as_ref().map(|s| s.to_string())),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MemberDocumentDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &MemberDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &MemberDocumentEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MemberDocumentEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active: Vec<MemberDocumentEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<MemberDocumentEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    async fn find_by_member_id(
        &self,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberDocumentEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let filtered: Vec<MemberDocumentEntity> = all_entities
            .iter()
            .filter(|e| e.member_id == member_id && e.deleted.is_none())
            .cloned()
            .collect();
        Ok(filtered.into())
    }

    async fn count_by_type(
        &self,
        document_type: &str,
        tx: Self::Transaction,
    ) -> Result<HashMap<Uuid, i64>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let mut counts: HashMap<Uuid, i64> = HashMap::new();
        for entity in all_entities.iter() {
            if entity.deleted.is_none() && entity.document_type.as_ref() == document_type {
                *counts.entry(entity.member_id).or_insert(0) += 1;
            }
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;

    fn make_document() -> MemberDocumentEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberDocumentEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            document_type: Arc::from("Beitrittserklärung"),
            description: Some(Arc::from("test doc")),
            file_name: Arc::from("beitritt.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("docs/beitritt.pdf"),
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
            // Phase 10: legacy fixture has all 3 mail-tracking fields NULL.
            template_id: None,
            mail_recipient_id: None,
            status: None,
        }
    }

    #[test]
    fn test_auditable_entity_type() {
        assert_eq!(MemberDocumentEntity::entity_type(), "member_document");
    }

    #[test]
    fn test_auditable_fields_count() {
        // Phase 10 D-07: extended from 6 to 9 fields (template_id, mail_recipient_id,
        // status appended at indices 6-8). Existing indices 0-5 unchanged (FROZEN-Order).
        let entity = make_document();
        let fields = entity.audit_fields();
        assert_eq!(fields.len(), 9);
        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_changes() {
        let old = make_document();
        let mut new = old.clone();
        new.description = Some(Arc::from("updated description"));

        let changes = old.diff(&new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field_name, "description");
    }

    #[test]
    fn test_auditable_diff_no_changes() {
        let entity = make_document();
        let changes = entity.diff(&entity);
        assert!(changes.is_empty());
    }

    // -------------------------------------------------------------------------
    // Phase 10 D-07 / D-08: FROZEN-Order tests for the 3 new mail-tracking
    // fields (template_id, mail_recipient_id, status) appended at indices 6-8.
    // -------------------------------------------------------------------------

    #[test]
    fn test_member_document_audit_fields_frozen_order_with_phase10_fields_present() {
        let now = time::OffsetDateTime::now_utc();
        let entity = MemberDocumentEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            document_type: Arc::from("repayment_mail"),
            description: Some(Arc::from("Subject")),
            file_name: Arc::from(""),
            mime_type: Arc::from("text/plain"),
            relative_path: Arc::from(""),
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: Uuid::new_v4(),
            template_id: Some(Uuid::new_v4()),
            mail_recipient_id: Some(Uuid::new_v4()),
            status: Some(Arc::from("sent")),
        };
        let fields = entity.audit_fields();
        assert_eq!(
            fields.len(),
            9,
            "audit_fields must contain exactly 9 entries (6 existing + 3 new mail-tracking fields)"
        );
        let names: Vec<&str> = fields.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "member_id",
                "document_type",
                "description",
                "file_name",
                "mime_type",
                "relative_path",
                "template_id",
                "mail_recipient_id",
                "status",
            ],
            "audit_fields order is FROZEN — new Phase 10 fields appended at indices 6-8"
        );
        assert!(fields[6].1.is_some(), "template_id must be Some when set");
        assert!(
            fields[7].1.is_some(),
            "mail_recipient_id must be Some when set"
        );
        assert_eq!(
            fields[8].1,
            Some("sent".to_string()),
            "status field must contain the literal 'sent' string"
        );
    }

    #[test]
    fn test_member_document_audit_fields_frozen_order_with_phase10_fields_none() {
        // Backward-compat: existing rows have NULL in the 3 new columns.
        // audit_fields() must still emit the 3 new entries at the end with None values,
        // so legacy entries that referenced only the first 6 fields remain hash-chain-stable.
        let now = time::OffsetDateTime::now_utc();
        let entity = MemberDocumentEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            document_type: Arc::from("join_declaration"),
            description: None,
            file_name: Arc::from("file.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("path/file.pdf"),
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: Uuid::new_v4(),
            template_id: None,
            mail_recipient_id: None,
            status: None,
        };
        let fields = entity.audit_fields();
        assert_eq!(fields.len(), 9);
        assert_eq!(fields[6].0, "template_id");
        assert!(
            fields[6].1.is_none(),
            "template_id NULL bleibt None im audit_fields"
        );
        assert_eq!(fields[7].0, "mail_recipient_id");
        assert!(fields[7].1.is_none(), "mail_recipient_id NULL bleibt None");
        assert_eq!(fields[8].0, "status");
        assert!(fields[8].1.is_none(), "status NULL bleibt None");
    }
}
