use async_trait::async_trait;
use genossi_dao::member::MemberEntity;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: Uuid,
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
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
    pub exit_date: Option<time::Date>,
    pub bank_account: Option<Arc<str>>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&MemberEntity> for Member {
    fn from(entity: &MemberEntity) -> Self {
        Self {
            id: entity.id,
            member_number: entity.member_number,
            first_name: entity.first_name.clone(),
            last_name: entity.last_name.clone(),
            email: entity.email.clone(),
            company: entity.company.clone(),
            comment: entity.comment.clone(),
            street: entity.street.clone(),
            house_number: entity.house_number.clone(),
            postal_code: entity.postal_code.clone(),
            city: entity.city.clone(),
            join_date: entity.join_date,
            shares_at_joining: entity.shares_at_joining,
            current_shares: entity.current_shares,
            current_balance: entity.current_balance,
            action_count: entity.action_count,
            exit_date: entity.exit_date,
            bank_account: entity.bank_account.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Member> for MemberEntity {
    fn from(member: &Member) -> Self {
        Self {
            id: member.id,
            member_number: member.member_number,
            first_name: member.first_name.clone(),
            last_name: member.last_name.clone(),
            email: member.email.clone(),
            company: member.company.clone(),
            comment: member.comment.clone(),
            street: member.street.clone(),
            house_number: member.house_number.clone(),
            postal_code: member.postal_code.clone(),
            city: member.city.clone(),
            join_date: member.join_date,
            shares_at_joining: member.shares_at_joining,
            current_shares: member.current_shares,
            current_balance: member.current_balance,
            action_count: member.action_count,
            exit_date: member.exit_date,
            bank_account: member.bank_account.clone(),
            created: member.created,
            deleted: member.deleted,
            version: member.version,
        }
    }
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MemberService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Member]>, ServiceError>;

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError>;

    async fn create(
        &self,
        item: &Member,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError>;

    async fn update(
        &self,
        item: &Member,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Member, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}
