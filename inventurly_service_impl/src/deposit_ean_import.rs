use async_trait::async_trait;
use csv::ReaderBuilder;
use std::sync::Arc;

use inventurly_dao::TransactionDao;
use inventurly_service::{
    deposit_ean_import::{
        CsvDepositEanRow, DepositEanImportError, DepositEanImportResult, DepositEanImportService,
    },
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    product::ProductService,
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct DepositEanImportServiceImpl: DepositEanImportService = DepositEanImportServiceDeps {
        ProductService: ProductService<Context = Self::Context, Transaction = Self::Transaction> = product_service,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

impl<Deps: DepositEanImportServiceDeps> DepositEanImportServiceImpl<Deps> {
    /// Parse CSV content and extract deposit EAN rows
    /// CSV format: semicolon-separated with columns EAN and PfandEAN
    fn parse_csv_content(&self, csv_content: &str) -> Result<Vec<(usize, CsvDepositEanRow)>, String> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b';')
            .from_reader(csv_content.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?;

        // Find column indices based on header names
        let ean_idx = headers
            .iter()
            .position(|h| h.trim() == "EAN" || h.trim() == "\u{feff}EAN") // Handle BOM
            .ok_or("EAN column not found")?;
        let pfand_ean_idx = headers
            .iter()
            .position(|h| h.trim() == "PfandEAN")
            .ok_or("PfandEAN column not found")?;

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

            let pfand_ean = record.get(pfand_ean_idx).unwrap_or("").trim();
            if pfand_ean.is_empty() {
                continue;
            }

            let row = CsvDepositEanRow {
                ean: ean.to_string(),
                pfand_ean: pfand_ean.to_string(),
            };

            rows.push((row_number, row));
        }

        Ok(rows)
    }
}

#[async_trait]
impl<Deps: DepositEanImportServiceDeps> DepositEanImportService for DepositEanImportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn import_deposit_eans_csv(
        &self,
        csv_content: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<DepositEanImportResult, ServiceError> {
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
            let pfand_ean = row.pfand_ean.clone();

            // Validate that the PfandEAN exists as a product
            match self
                .product_service
                .get_by_ean(&pfand_ean, context.clone(), Some(tx.clone()))
                .await
            {
                Ok(_) => {
                    // PfandEAN exists, continue
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    errors.push(DepositEanImportError {
                        row: row_number,
                        ean: ean.clone(),
                        message: format!("Deposit product not found with EAN '{}'", pfand_ean),
                    });
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    errors.push(DepositEanImportError {
                        row: row_number,
                        ean: ean.clone(),
                        message: format!("Error looking up deposit product: {:?}", e),
                    });
                    skipped += 1;
                    continue;
                }
            }

            // Look up product by EAN
            match self
                .product_service
                .get_by_ean(&ean, context.clone(), Some(tx.clone()))
                .await
            {
                Ok(mut product) => {
                    // Update the deposit_ean
                    product.deposit_ean = Some(Arc::from(pfand_ean.as_str()));

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
                            errors.push(DepositEanImportError {
                                row: row_number,
                                ean,
                                message: format!("Failed to update product: {:?}", e),
                            });
                            skipped += 1;
                        }
                    }
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    errors.push(DepositEanImportError {
                        row: row_number,
                        ean,
                        message: "Product not found".to_string(),
                    });
                    skipped += 1;
                }
                Err(e) => {
                    errors.push(DepositEanImportError {
                        row: row_number,
                        ean,
                        message: format!("Error looking up product: {:?}", e),
                    });
                    skipped += 1;
                }
            }
        }

        self.transaction_dao.commit(tx).await?;

        Ok(DepositEanImportResult {
            total_rows,
            updated,
            skipped,
            errors,
        })
    }
}
