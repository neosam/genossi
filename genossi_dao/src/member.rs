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
#[derive(Default)]
pub enum MemberStatus {
    #[default]
    Normal,
    FehlerhaftErfasst,
}

impl MemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberStatus::Normal => "Normal",
            MemberStatus::FehlerhaftErfasst => "FehlerhaftErfasst",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Normal" => Ok(MemberStatus::Normal),
            "FehlerhaftErfasst" => Ok(MemberStatus::FehlerhaftErfasst),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown member status: {}",
                s
            )))),
        }
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, MemberStatus::Normal)
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
    pub status: MemberStatus,
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

    async fn count_active(
        &self,
        today: time::Date,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let count = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .filter(|e| e.status.is_normal())
            .filter(|e| e.exit_date.is_none_or(|d| d > today))
            .count();
        Ok(count as u64)
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

impl crate::auditable::Auditable for MemberEntity {
    fn entity_type() -> &'static str {
        "member"
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
            ("member_number", Some(self.member_number.to_string())),
            ("first_name", Some(self.first_name.to_string())),
            ("last_name", Some(self.last_name.to_string())),
            (
                "salutation",
                self.salutation.as_ref().map(|s| s.as_str().to_string()),
            ),
            ("title", self.title.as_ref().map(|s| s.to_string())),
            ("email", self.email.as_ref().map(|s| s.to_string())),
            ("company", self.company.as_ref().map(|s| s.to_string())),
            ("comment", self.comment.as_ref().map(|s| s.to_string())),
            ("street", self.street.as_ref().map(|s| s.to_string())),
            (
                "house_number",
                self.house_number.as_ref().map(|s| s.to_string()),
            ),
            (
                "postal_code",
                self.postal_code.as_ref().map(|s| s.to_string()),
            ),
            ("city", self.city.as_ref().map(|s| s.to_string())),
            ("join_date", Some(format_date(&self.join_date))),
            (
                "shares_at_joining",
                Some(self.shares_at_joining.to_string()),
            ),
            ("current_shares", Some(self.current_shares.to_string())),
            ("current_balance", Some(self.current_balance.to_string())),
            ("action_count", Some(self.action_count.to_string())),
            ("migrated", Some(self.migrated.to_string())),
            ("exit_date", self.exit_date.as_ref().map(format_date)),
            (
                "bank_account",
                self.bank_account.as_ref().map(|s| s.to_string()),
            ),
            ("status", Some(self.status.as_str().to_string())),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockTransaction;

    fn make_entity(member_number: i64, deleted: Option<time::PrimitiveDateTime>) -> MemberEntity {
        make_entity_with_exit(member_number, deleted, None)
    }

    fn make_entity_with_exit(
        member_number: i64,
        deleted: Option<time::PrimitiveDateTime>,
        exit_date: Option<time::Date>,
    ) -> MemberEntity {
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
            exit_date,
            bank_account: None,
            status: MemberStatus::Normal,
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

    #[tokio::test]
    async fn test_count_active_all_active() {
        let dao = TestMemberDao {
            entities: Arc::from(vec![
                make_entity(1, None),
                make_entity(2, None),
                make_entity(3, None),
            ]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_count_active_excludes_deleted() {
        let deleted_at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2025, time::Month::March, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        let dao = TestMemberDao {
            entities: Arc::from(vec![make_entity(1, None), make_entity(2, Some(deleted_at))]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_count_active_excludes_past_exit_date() {
        let past_exit = time::Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
        let dao = TestMemberDao {
            entities: Arc::from(vec![
                make_entity(1, None),
                make_entity_with_exit(2, None, Some(past_exit)),
            ]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_count_active_includes_future_exit_date() {
        let future_exit = time::Date::from_calendar_date(2025, time::Month::December, 31).unwrap();
        let dao = TestMemberDao {
            entities: Arc::from(vec![
                make_entity(1, None),
                make_entity_with_exit(2, None, Some(future_exit)),
            ]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_active_empty() {
        let dao = TestMemberDao {
            entities: Arc::from(vec![]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_active_exit_date_today_not_counted() {
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let dao = TestMemberDao {
            entities: Arc::from(vec![make_entity_with_exit(1, None, Some(today))]),
        };
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 0);
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

    #[test]
    fn test_member_status_roundtrip() {
        for variant in &[MemberStatus::Normal, MemberStatus::FehlerhaftErfasst] {
            let s = variant.as_str();
            let parsed = MemberStatus::from_str(s).unwrap();
            assert_eq!(&parsed, variant);
        }
    }

    #[test]
    fn test_member_status_as_str() {
        assert_eq!(MemberStatus::Normal.as_str(), "Normal");
        assert_eq!(
            MemberStatus::FehlerhaftErfasst.as_str(),
            "FehlerhaftErfasst"
        );
    }

    #[test]
    fn test_member_status_invalid_value() {
        let result = MemberStatus::from_str("Invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_member_status_default() {
        assert_eq!(MemberStatus::default(), MemberStatus::Normal);
    }

    #[test]
    fn test_member_status_is_normal() {
        assert!(MemberStatus::Normal.is_normal());
        assert!(!MemberStatus::FehlerhaftErfasst.is_normal());
    }

    #[tokio::test]
    async fn test_count_active_excludes_fehlerhaft_erfasst() {
        let mut entity = make_entity(1, None);
        entity.status = MemberStatus::FehlerhaftErfasst;
        let dao = TestMemberDao {
            entities: Arc::from(vec![make_entity(2, None), entity]),
        };
        let today = time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap();
        let count = dao.count_active(today, mock_tx()).await.unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_auditable_entity_type() {
        use crate::auditable::Auditable;
        assert_eq!(MemberEntity::entity_type(), "member");
    }

    #[test]
    fn test_auditable_fields_count() {
        use crate::auditable::Auditable;
        let entity = make_entity(1, None);
        let fields = entity.audit_fields();
        assert_eq!(fields.len(), 21);
        // Verify excluded fields are not present
        let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
        assert!(!field_names.contains(&"id"));
        assert!(!field_names.contains(&"version"));
        assert!(!field_names.contains(&"created"));
        assert!(!field_names.contains(&"deleted"));
    }

    #[test]
    fn test_auditable_diff_detects_changes() {
        use crate::auditable::Auditable;
        let mut old = make_entity(1, None);
        old.first_name = Arc::from("Alice");
        old.email = Some(Arc::from("alice@example.com"));
        let mut new = old.clone();
        new.first_name = Arc::from("Bob");
        new.email = Some(Arc::from("bob@example.com"));

        let changes = old.diff(&new);
        assert_eq!(changes.len(), 2);
        let names: Vec<&str> = changes.iter().map(|c| c.field_name).collect();
        assert!(names.contains(&"first_name"));
        assert!(names.contains(&"email"));
    }

    #[test]
    fn test_auditable_diff_no_changes() {
        use crate::auditable::Auditable;
        let entity = make_entity(1, None);
        let changes = entity.diff(&entity);
        assert!(changes.is_empty());
    }
}
