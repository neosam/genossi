use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::member::Salutation;
use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationStatus {
    Offen,
    Bestaetigt,
    Abgelehnt,
}

impl ApplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationStatus::Offen => "Offen",
            ApplicationStatus::Bestaetigt => "Bestaetigt",
            ApplicationStatus::Abgelehnt => "Abgelehnt",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Offen" => Ok(ApplicationStatus::Offen),
            "Bestaetigt" => Ok(ApplicationStatus::Bestaetigt),
            "Abgelehnt" => Ok(ApplicationStatus::Abgelehnt),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown application status: {}",
                s
            )))),
        }
    }
}

impl Default for ApplicationStatus {
    fn default() -> Self {
        ApplicationStatus::Offen
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationEntity {
    pub id: Uuid,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub shares: i32,
    pub status: ApplicationStatus,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait ApplicationDao {
    type Transaction: crate::Transaction;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[ApplicationEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &ApplicationEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &ApplicationEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[ApplicationEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ApplicationEntity> = all_entities
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
    ) -> Result<Option<ApplicationEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }
}
