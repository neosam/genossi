use async_trait::async_trait;
use csv::ReaderBuilder;
use std::sync::Arc;

use inventurly_dao::TransactionDao;
use inventurly_service::{
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    price_import::{CsvPriceRow, PriceImportError, PriceImportResult, PriceImportService},
    product::{Price, ProductService},
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct PriceImportServiceImpl: PriceImportService = PriceImportServiceDeps {
        ProductService: ProductService<Context = Self::Context, Transaction = Self::Transaction> = product_service,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

impl<Deps: PriceImportServiceDeps> PriceImportServiceImpl<Deps> {
    /// Parse CSV content and extract price rows
    fn parse_csv_content(&self, csv_content: &str) -> Result<Vec<(usize, CsvPriceRow)>, String> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_content.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?;

        // Find column indices based on header names
        let ean_idx = headers
            .iter()
            .position(|h| h == "EAN")
            .ok_or("EAN column not found")?;
        let ekn_idx = headers
            .iter()
            .position(|h| h == "EKN")
            .ok_or("EKN column not found")?;

        let mut rows = Vec::new();

        for (row_index, result) in reader.records().enumerate() {
            let row_number = row_index + 2; // +2 because CSV has header and is 1-indexed
            let record =
                result.map_err(|e| format!("Failed to read row {}: {}", row_number, e))?;

            // Skip rows with empty EAN
            let ean = record.get(ean_idx).unwrap_or("").trim();
            if ean.is_empty() {
                continue;
            }

            let row = CsvPriceRow {
                ean: ean.to_string(),
                ekn: record.get(ekn_idx).unwrap_or("0").trim().to_string(),
            };

            rows.push((row_number, row));
        }

        Ok(rows)
    }
}

#[async_trait]
impl<Deps: PriceImportServiceDeps> PriceImportService for PriceImportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn import_prices_csv(
        &self,
        csv_content: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<PriceImportResult, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Check permissions
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        // Parse CSV content
        let rows = self.parse_csv_content(csv_content).map_err(|e| {
            ServiceError::ValidationError(vec![inventurly_service::ValidationFailureItem {
                field: Arc::from("csv"),
                message: Arc::from(e),
            }])
        })?;

        let total_rows = rows.len();
        let mut updated = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        // Process each row
        for (row_number, row) in rows {
            let ean = row.ean.clone();

            // Parse the price
            let price_cents = match row.parse_price() {
                Ok(cents) => cents,
                Err(e) => {
                    errors.push(PriceImportError {
                        row: row_number,
                        ean,
                        message: e,
                    });
                    skipped += 1;
                    continue;
                }
            };

            // Look up product by EAN
            match self
                .product_service
                .get_by_ean(&ean, context.clone(), Some(tx.clone()))
                .await
            {
                Ok(mut product) => {
                    // Update the price
                    product.price = Price::from_cents(price_cents);

                    // Save the updated product
                    match self
                        .product_service
                        .update(&product, context.clone(), Some(tx.clone()))
                        .await
                    {
                        Ok(_) => {
                            updated += 1;
                        }
                        Err(e) => {
                            errors.push(PriceImportError {
                                row: row_number,
                                ean,
                                message: format!("Failed to update product: {:?}", e),
                            });
                            skipped += 1;
                        }
                    }
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    errors.push(PriceImportError {
                        row: row_number,
                        ean,
                        message: "Product not found".to_string(),
                    });
                    skipped += 1;
                }
                Err(e) => {
                    errors.push(PriceImportError {
                        row: row_number,
                        ean,
                        message: format!("Error looking up product: {:?}", e),
                    });
                    skipped += 1;
                }
            }
        }

        self.transaction_dao.commit(tx).await?;

        Ok(PriceImportResult {
            total_rows,
            updated,
            skipped,
            errors,
        })
    }
}
