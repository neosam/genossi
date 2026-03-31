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
