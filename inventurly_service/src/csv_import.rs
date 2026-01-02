use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;

use crate::{permission::Authentication, product::Product, ServiceError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CsvImportResult {
    pub total_rows: usize,
    pub created: usize,
    pub updated: usize,
    pub reactivated: usize,
    pub deleted: usize,
    pub errors: Vec<CsvImportError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CsvImportError {
    pub row: usize,
    pub ean: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CsvProductRow {
    pub ean: String,
    pub bezeichnung: String,
    pub kurzbezeichnung: String,
    pub vk_einheit: String,
    pub wiege_artikel: String,
    pub vk_herst: String,
}

impl CsvProductRow {
    /// Parse price from German format ("5,39") to euros as f64
    pub fn parse_price(&self) -> Result<f64, String> {
        if self.vk_herst.is_empty() || self.vk_herst == "0" || self.vk_herst == "0,00" {
            return Ok(0.0);
        }

        let normalized = self.vk_herst.replace(',', ".");
        normalized
            .parse::<f64>()
            .map_err(|e| format!("Invalid price format '{}': {}", self.vk_herst, e))
    }

    /// Parse weighing requirement from German format ("9" = true, "0" = false)
    pub fn parse_requires_weighing(&self) -> Result<bool, String> {
        match self.wiege_artikel.as_str() {
            "9" => Ok(true),
            "0" => Ok(false),
            _ => Err(format!(
                "Invalid WiegeArtikel value '{}', expected '0' or '9'",
                self.wiege_artikel
            )),
        }
    }

    /// Validate required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.ean.is_empty() {
            return Err("EAN cannot be empty".to_string());
        }
        if self.bezeichnung.is_empty() {
            return Err("Bezeichnung cannot be empty".to_string());
        }
        if self.kurzbezeichnung.is_empty() {
            return Err("Kurzbezeichnung cannot be empty".to_string());
        }
        if self.vk_einheit.is_empty() {
            return Err("VKEinheit cannot be empty".to_string());
        }

        // Validate price parsing
        self.parse_price()?;

        // Validate weighing requirement parsing
        self.parse_requires_weighing()?;

        Ok(())
    }
}

impl TryFrom<CsvProductRow> for Product {
    type Error = String;

    fn try_from(row: CsvProductRow) -> Result<Self, Self::Error> {
        row.validate()?;

        let price_euros = row.parse_price()?;
        let requires_weighing = row.parse_requires_weighing()?;

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        Ok(Product {
            id: uuid::Uuid::new_v4(),
            ean: Arc::from(row.ean.as_str()),
            name: Arc::from(row.bezeichnung.as_str()),
            short_name: Arc::from(row.kurzbezeichnung.as_str()),
            sales_unit: Arc::from(row.vk_einheit.as_str()),
            requires_weighing,
            price: crate::product::Price::from_euros(price_euros),
            created,
            deleted: None,
            version: uuid::Uuid::new_v4(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImportAction {
    Created,
    Updated,
    Reactivated,
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait CsvImportService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    /// Import products from CSV content
    /// If `remove_unlisted` is true, products not in the CSV will be soft-deleted
    async fn import_products_csv(
        &self,
        csv_content: &str,
        remove_unlisted: bool,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<CsvImportResult, ServiceError>;

    /// Import a single product row (for internal use)
    async fn import_product_row(
        &self,
        row: CsvProductRow,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ImportAction, ServiceError>;
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
