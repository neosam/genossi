use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

/// Aggregated product measurement data for an inventur report
#[derive(Debug, Clone)]
pub struct InventurProductReportItem {
    pub ean: Arc<str>,
    pub product_name: Arc<str>,
    pub short_name: Arc<str>,
    pub total_count: Option<i64>,
    pub total_weight_grams: Option<i64>,
    pub measurement_count: usize,
    pub racks_measured: Vec<Arc<str>>,
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait InventurReportService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    /// Generate cumulative product report for an inventur
    /// Aggregates all measurements and custom entries (with EAN) by product EAN
    async fn get_product_report(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurProductReportItem]>, ServiceError>;

    /// Generate CSV string for the product report
    async fn get_product_report_csv(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<String, ServiceError>;
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
