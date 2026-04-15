use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionType {
    Eintritt,
    Austritt,
    Todesfall,
    Aufstockung,
    Verkauf,
    UebertragungEmpfang,
    UebertragungAbgabe,
    Note,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Eintritt => "Eintritt",
            ActionType::Austritt => "Austritt",
            ActionType::Todesfall => "Todesfall",
            ActionType::Aufstockung => "Aufstockung",
            ActionType::Verkauf => "Verkauf",
            ActionType::UebertragungEmpfang => "UebertragungEmpfang",
            ActionType::UebertragungAbgabe => "UebertragungAbgabe",
            ActionType::Note => "Note",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Eintritt" => Ok(ActionType::Eintritt),
            "Austritt" => Ok(ActionType::Austritt),
            "Todesfall" => Ok(ActionType::Todesfall),
            "Aufstockung" => Ok(ActionType::Aufstockung),
            "Verkauf" => Ok(ActionType::Verkauf),
            "UebertragungEmpfang" => Ok(ActionType::UebertragungEmpfang),
            "UebertragungAbgabe" => Ok(ActionType::UebertragungAbgabe),
            "Note" => Ok(ActionType::Note),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown action type: {}",
                s
            )))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberActionEntity {
    pub id: Uuid,
    pub member_id: Uuid,
    pub action_type: ActionType,
    pub date: time::Date,
    pub shares_change: i32,
    pub transfer_member_id: Option<Uuid>,
    pub effective_date: Option<time::Date>,
    pub comment: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl crate::auditable::Auditable for MemberActionEntity {
    fn entity_type() -> &'static str {
        "member_action"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        let format_date = |d: &time::Date| {
            let fmt = time::format_description::parse("[year]-[month]-[day]").unwrap();
            d.format(&fmt).unwrap()
        };
        vec![
            ("member_id", Some(self.member_id.to_string())),
            ("action_type", Some(self.action_type.as_str().to_string())),
            ("date", Some(format_date(&self.date))),
            ("shares_change", Some(self.shares_change.to_string())),
            ("transfer_member_id", self.transfer_member_id.map(|u| u.to_string())),
            ("effective_date", self.effective_date.as_ref().map(format_date)),
            ("comment", self.comment.as_ref().map(|s| s.to_string())),
        ]
    }
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MemberActionDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[MemberActionEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &MemberActionEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &MemberActionEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MemberActionEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<MemberActionEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<MemberActionEntity>, DaoError> {
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
    ) -> Result<Arc<[MemberActionEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let filtered: Vec<MemberActionEntity> = all_entities
            .iter()
            .filter(|e| e.member_id == member_id && e.deleted.is_none())
            .cloned()
            .collect();
        Ok(filtered.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;

    fn make_action() -> MemberActionEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberActionEntity {
            id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            action_type: ActionType::Aufstockung,
            date,
            shares_change: 5,
            transfer_member_id: None,
            effective_date: None,
            comment: Some(Arc::from("test")),
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_auditable_entity_type() {
        assert_eq!(MemberActionEntity::entity_type(), "member_action");
    }

    #[test]
    fn test_auditable_fields_count() {
        let entity = make_action();
        let fields = entity.audit_fields();
        assert_eq!(fields.len(), 7);
        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_changes() {
        let old = make_action();
        let mut new = old.clone();
        new.shares_change = 10;
        new.comment = Some(Arc::from("updated"));

        let changes = old.diff(&new);
        assert_eq!(changes.len(), 2);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert!(names.contains(&"shares_change"));
        assert!(names.contains(&"comment"));
    }

    #[test]
    fn test_auditable_diff_no_changes() {
        let entity = make_action();
        let changes = entity.diff(&entity);
        assert!(changes.is_empty());
    }
}
