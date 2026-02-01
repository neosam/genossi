use async_trait::async_trait;
use mockall::automock;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PriceImportResult {
    pub total_rows: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<PriceImportError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PriceImportError {
    pub row: usize,
    pub ean: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CsvPriceRow {
    pub ean: String,
    pub ekn: String,
}

impl CsvPriceRow {
    /// Parse EKN price from German format (e.g., "2,50 €" or "2.50") to cents
    pub fn parse_price(&self) -> Result<i64, String> {
        parse_ekn_price(&self.ekn)
    }
}

/// Parse EKN price string to cents
/// Handles:
/// - Euro symbol (€) removal
/// - German decimal format (comma as decimal separator)
/// - Period decimal format
/// - Thousands separators
pub fn parse_ekn_price(s: &str) -> Result<i64, String> {
    let trimmed = s.trim();

    // Handle completely empty string
    if trimmed.is_empty() {
        return Ok(0);
    }

    // Remove € symbol and whitespace
    let cleaned = trimmed.replace('€', "").trim().to_string();

    // After removing €, if nothing left, that's an error (just "€" is invalid)
    if cleaned.is_empty() {
        return Err(format!("Invalid price format '{}': no numeric value", s));
    }

    if cleaned == "0" {
        return Ok(0);
    }

    // Determine decimal separator and normalize
    let normalized = if cleaned.contains(',') && !cleaned.contains('.') {
        // German format: 2,50
        cleaned.replace(',', ".")
    } else if cleaned.contains('.') && cleaned.contains(',') {
        // Thousands separator case: 1.234,56 -> remove . and replace ,
        cleaned.replace('.', "").replace(',', ".")
    } else {
        // Already in standard format: 2.50
        cleaned
    };

    let euros: f64 = normalized
        .parse()
        .map_err(|e| format!("Invalid price format '{}': {}", s, e))?;

    Ok((euros * 100.0).round() as i64)
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait PriceImportService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    /// Import prices from CSV content
    /// Updates existing products' prices based on EAN lookup
    /// Products not found are skipped and reported as errors
    async fn import_prices_csv(
        &self,
        csv_content: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<PriceImportResult, ServiceError>;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ekn_price_german_format_with_euro() {
        assert_eq!(parse_ekn_price("2,50 €").unwrap(), 250);
        assert_eq!(parse_ekn_price("5,39 €").unwrap(), 539);
        assert_eq!(parse_ekn_price("30,40 €").unwrap(), 3040);
    }

    #[test]
    fn test_parse_ekn_price_german_format_without_euro() {
        assert_eq!(parse_ekn_price("2,50").unwrap(), 250);
        assert_eq!(parse_ekn_price("5,39").unwrap(), 539);
    }

    #[test]
    fn test_parse_ekn_price_period_format() {
        assert_eq!(parse_ekn_price("2.50").unwrap(), 250);
        assert_eq!(parse_ekn_price("5.39").unwrap(), 539);
    }

    #[test]
    fn test_parse_ekn_price_thousands_separator() {
        assert_eq!(parse_ekn_price("1.234,56").unwrap(), 123456);
        assert_eq!(parse_ekn_price("1.234,56 €").unwrap(), 123456);
    }

    #[test]
    fn test_parse_ekn_price_zero_and_empty() {
        assert_eq!(parse_ekn_price("0").unwrap(), 0);
        assert_eq!(parse_ekn_price("").unwrap(), 0);
        assert_eq!(parse_ekn_price("  ").unwrap(), 0);
    }

    #[test]
    fn test_parse_ekn_price_invalid() {
        assert!(parse_ekn_price("abc").is_err());
        assert!(parse_ekn_price("€").is_err());
    }
}
