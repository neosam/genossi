pub mod person;
pub mod product;
pub mod permission;

use std::sync::Arc;
use async_trait::async_trait;
use mockall::automock;

#[derive(Debug, Clone)]
pub enum DaoError {
    DatabaseError(Arc<str>),
    ParseError(Arc<str>),
    NotFound,
    ConflictError(Arc<str>),
}

impl From<uuid::Error> for DaoError {
    fn from(e: uuid::Error) -> Self {
        DaoError::ParseError(Arc::from(e.to_string()))
    }
}

impl From<time::error::Parse> for DaoError {
    fn from(e: time::error::Parse) -> Self {
        DaoError::ParseError(Arc::from(e.to_string()))
    }
}

#[async_trait]
pub trait Transaction: Clone + Send + Sync + 'static {
    async fn begin(&mut self) -> Result<(), DaoError>;
    async fn commit(self) -> Result<(), DaoError>;
    async fn rollback(self) -> Result<(), DaoError>;
}

mockall::mock! {
    pub Transaction {}
    
    impl Clone for Transaction {
        fn clone(&self) -> Self;
    }
    
    #[async_trait]
    impl Transaction for Transaction {
        async fn begin(&mut self) -> Result<(), DaoError>;
        async fn commit(self) -> Result<(), DaoError>;
        async fn rollback(self) -> Result<(), DaoError>;
    }
}

#[automock(type Transaction = MockTransaction;)]
#[async_trait]
pub trait TransactionDao {
    type Transaction: Transaction;
    
    async fn transaction(&self) -> Result<Self::Transaction, DaoError>;
    async fn use_transaction(&self, tx: Option<Self::Transaction>) -> Result<Self::Transaction, DaoError>;
    async fn commit(&self, tx: Self::Transaction) -> Result<(), DaoError>;
}