use async_trait::async_trait;
use mockall::automock;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DepositEanImportResult {
    pub total_rows: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<DepositEanImportError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct DepositEanImportError {
    pub row: usize,
    pub ean: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CsvDepositEanRow {
    pub ean: String,
    pub pfand_ean: String,
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait DepositEanImportService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    /// Import deposit EANs from CSV content
    /// Updates existing products' deposit_ean based on EAN lookup
    /// Products not found are skipped and reported as errors
    /// PfandEAN values that don't exist as products are also skipped
    async fn import_deposit_eans_csv(
        &self,
        csv_content: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<DepositEanImportResult, ServiceError>;
}

mockall::mock! {
    pub Context {}
    impl Clone for Context {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Context {}
    unsafe impl Sync for Context {}
}

mockall::mock! {
    pub Transaction {}
    impl Clone for Transaction {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Transaction {}
    unsafe impl Sync for Transaction {}
}
