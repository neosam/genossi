use async_trait::async_trait;
use inventurly_dao::{DaoError, Transaction, TransactionDao};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct TransactionImpl {
    pub tx: Arc<Mutex<sqlx::Transaction<'static, sqlx::Sqlite>>>,
    is_committed: Arc<Mutex<bool>>,
}

impl TransactionImpl {
    pub async fn new(pool: &SqlitePool) -> Result<Self, DaoError> {
        let tx = pool
            .begin()
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(Self {
            tx: Arc::new(Mutex::new(tx)),
            is_committed: Arc::new(Mutex::new(false)),
        })
    }
}

#[async_trait]
impl Transaction for TransactionImpl {
    async fn begin(&mut self) -> Result<(), DaoError> {
        Ok(())
    }

    async fn commit(self) -> Result<(), DaoError> {
        let mut is_committed = self.is_committed.lock().await;
        if !*is_committed && Arc::strong_count(&self.tx) == 1 {
            let tx = Arc::try_unwrap(self.tx)
                .map_err(|_| DaoError::DatabaseError(Arc::from("Cannot unwrap transaction")))?
                .into_inner();
            tx.commit()
                .await
                .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
            *is_committed = true;
        }
        Ok(())
    }

    async fn rollback(self) -> Result<(), DaoError> {
        if Arc::strong_count(&self.tx) == 1 {
            let tx = Arc::try_unwrap(self.tx)
                .map_err(|_| DaoError::DatabaseError(Arc::from("Cannot unwrap transaction")))?
                .into_inner();
            tx.rollback()
                .await
                .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        }
        Ok(())
    }
}

pub struct TransactionDaoImpl {
    pool: Arc<SqlitePool>,
}

impl TransactionDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionDao for TransactionDaoImpl {
    type Transaction = TransactionImpl;

    async fn transaction(&self) -> Result<Self::Transaction, DaoError> {
        TransactionImpl::new(&self.pool).await
    }

    async fn use_transaction(
        &self,
        tx: Option<Self::Transaction>,
    ) -> Result<Self::Transaction, DaoError> {
        match tx {
            Some(tx) => Ok(tx),
            None => self.transaction().await,
        }
    }

    async fn commit(&self, tx: Self::Transaction) -> Result<(), DaoError> {
        tx.commit().await
    }
}
