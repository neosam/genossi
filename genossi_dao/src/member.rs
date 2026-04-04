use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Salutation {
    Herr,
    Frau,
    Firma,
}

impl Salutation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Salutation::Herr => "Herr",
            Salutation::Frau => "Frau",
            Salutation::Firma => "Firma",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Herr" => Ok(Salutation::Herr),
            "Frau" => Ok(Salutation::Frau),
            "Firma" => Ok(Salutation::Firma),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown salutation: {}",
                s
            )))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberEntity {
    pub id: Uuid,
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub company: Option<Arc<str>>,
    pub comment: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub join_date: time::Date,
    pub shares_at_joining: i32,
    pub current_shares: i32,
    pub current_balance: i64,
    pub action_count: i32,
    pub migrated: bool,
    pub exit_date: Option<time::Date>,
    pub bank_account: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait MemberDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &MemberEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &MemberEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<MemberEntity> = all_entities
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
    ) -> Result<Option<MemberEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }

    async fn update_migrated(
        &self,
        id: Uuid,
        migrated: bool,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update_dates(
        &self,
        id: Uuid,
        join_date: time::Date,
        exit_date: Option<time::Date>,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn find_by_member_number(
        &self,
        member_number: i64,
        tx: Self::Transaction,
    ) -> Result<Option<MemberEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.member_number == member_number && e.deleted.is_none())
            .cloned())
    }

    async fn next_member_number(&self, tx: Self::Transaction) -> Result<i64, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let max = all_entities
            .iter()
            .map(|e| e.member_number)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockTransaction;

    fn make_entity(member_number: i64, deleted: Option<time::PrimitiveDateTime>) -> MemberEntity {
        let date = time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        MemberEntity {
            id: Uuid::new_v4(),
            member_number,
            first_name: Arc::from("Test"),
            last_name: Arc::from("User"),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            created: datetime,
            deleted,
            version: Uuid::new_v4(),
        }
    }

    struct TestMemberDao {
        entities: Arc<[MemberEntity]>,
    }

    #[async_trait]
    impl MemberDao for TestMemberDao {
        type Transaction = MockTransaction;

        async fn dump_all(&self, _tx: Self::Transaction) -> Result<Arc<[MemberEntity]>, DaoError> {
            Ok(self.entities.clone())
        }

        async fn create(
            &self,
            _entity: &MemberEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update(
            &self,
            _entity: &MemberEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update_migrated(
            &self,
            _id: Uuid,
            _migrated: bool,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update_dates(
            &self,
            _id: Uuid,
            _join_date: time::Date,
            _exit_date: Option<time::Date>,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }
    }

    fn mock_tx() -> MockTransaction {
        let mut tx = MockTransaction::new();
        tx.expect_clone().returning(MockTransaction::new);
        tx
    }

    #[tokio::test]
    async fn test_next_member_number_empty() {
        let dao = TestMemberDao {
            entities: Arc::from(vec![]),
        };
        let result = dao.next_member_number(mock_tx()).await.unwrap();
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_next_member_number_with_members() {
        let dao = TestMemberDao {
            entities: Arc::from(vec![
                make_entity(5, None),
                make_entity(10, None),
                make_entity(3, None),
            ]),
        };
        let result = dao.next_member_number(mock_tx()).await.unwrap();
        assert_eq!(result, 11);
    }

    #[tokio::test]
    async fn test_next_member_number_includes_soft_deleted() {
        let deleted_at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        let dao = TestMemberDao {
            entities: Arc::from(vec![
                make_entity(5, None),
                make_entity(100, Some(deleted_at)),
            ]),
        };
        let result = dao.next_member_number(mock_tx()).await.unwrap();
        assert_eq!(result, 101);
    }

    #[test]
    fn test_salutation_roundtrip() {
        for variant in &[Salutation::Herr, Salutation::Frau, Salutation::Firma] {
            let s = variant.as_str();
            let parsed = Salutation::from_str(s).unwrap();
            assert_eq!(&parsed, variant);
        }
    }

    #[test]
    fn test_salutation_as_str() {
        assert_eq!(Salutation::Herr.as_str(), "Herr");
        assert_eq!(Salutation::Frau.as_str(), "Frau");
        assert_eq!(Salutation::Firma.as_str(), "Firma");
    }

    #[test]
    fn test_salutation_invalid_value() {
        let result = Salutation::from_str("Invalid");
        assert!(result.is_err());
    }
}
