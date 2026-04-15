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

impl crate::auditable::Auditable for ApplicationEntity {
    fn entity_type() -> &'static str {
        "application"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("first_name", Some(self.first_name.to_string())),
            ("last_name", Some(self.last_name.to_string())),
            ("salutation", self.salutation.as_ref().map(|s| s.as_str().to_string())),
            ("title", self.title.as_ref().map(|s| s.to_string())),
            ("email", self.email.as_ref().map(|s| s.to_string())),
            ("street", self.street.as_ref().map(|s| s.to_string())),
            ("house_number", self.house_number.as_ref().map(|s| s.to_string())),
            ("postal_code", self.postal_code.as_ref().map(|s| s.to_string())),
            ("city", self.city.as_ref().map(|s| s.to_string())),
            ("shares", Some(self.shares.to_string())),
            ("status", Some(self.status.as_str().to_string())),
        ]
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auditable::Auditable;
    use crate::member::Salutation;

    fn make_application() -> ApplicationEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::April, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        ApplicationEntity {
            id: Uuid::new_v4(),
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: Some(Salutation::Herr),
            title: None,
            email: Some(Arc::from("max@example.com")),
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            shares: 1,
            status: ApplicationStatus::Offen,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_auditable_entity_type() {
        assert_eq!(ApplicationEntity::entity_type(), "application");
    }

    #[test]
    fn test_auditable_fields_count() {
        let entity = make_application();
        let fields = entity.audit_fields();
        assert_eq!(fields.len(), 11);
        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_changes() {
        let old = make_application();
        let mut new = old.clone();
        new.status = ApplicationStatus::Bestaetigt;
        new.shares = 3;

        let changes = old.diff(&new);
        assert_eq!(changes.len(), 2);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert!(names.contains(&"status"));
        assert!(names.contains(&"shares"));
    }

    #[test]
    fn test_auditable_diff_no_changes() {
        let entity = make_application();
        let changes = entity.diff(&entity);
        assert!(changes.is_empty());
    }
}
