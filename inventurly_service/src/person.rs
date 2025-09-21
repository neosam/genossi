use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: Uuid,
    pub name: Arc<str>,
    pub age: i32,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::person::PersonEntity> for Person {
    fn from(entity: &inventurly_dao::person::PersonEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
            age: entity.age,
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Person> for inventurly_dao::person::PersonEntity {
    fn from(person: &Person) -> Self {
        Self {
            id: person.id,
            name: person.name.clone(),
            age: person.age,
            created: person.created,
            deleted: person.deleted,
            version: person.version,
        }
    }
}

#[automock(type Context=(); type Transaction = inventurly_dao::MockTransaction;)]
#[async_trait]
pub trait PersonService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: inventurly_dao::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Person]>, ServiceError>;

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError>;

    async fn create(
        &self,
        item: &Person,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError>;

    async fn update(
        &self,
        item: &Person,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError>;

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
}