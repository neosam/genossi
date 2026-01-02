use axum::body::Body;
use axum::extract::{Multipart, Query, State};
use axum::response::Response;
use axum::routing::post;
use axum::{Extension, Router};
use inventurly_service::csv_import::{CsvImportError, CsvImportResult, CsvImportService};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi};

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(Debug, Deserialize, IntoParams)]
pub struct CsvImportQuery {
    /// If true, products not in the CSV will be soft-deleted
    #[serde(default)]
    pub remove_unlisted: bool,
}

/// Schema for CSV file upload in multipart form
#[derive(utoipa::ToSchema)]
#[schema(title = "CsvFileUpload")]
pub struct CsvFileUpload {
    /// CSV file to upload
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/products", post(import_products_csv::<RestState>))
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "CSV Import",
    path = "/products",
    params(CsvImportQuery),
    request_body(
        content = inline(CsvFileUpload),
        description = "CSV file upload",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "CSV import successful", body = CsvImportResult),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn import_products_csv<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<CsvImportQuery>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            // Extract the CSV file from multipart form data
            let mut csv_content = String::new();

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Multipart parsing error: {}", e)))?
            {
                let field_name = field.name().unwrap_or("unknown");

                if field_name == "file" {
                    let file_data = field.bytes().await.map_err(|e| {
                        RestError::BadRequest(format!("Failed to read file: {}", e))
                    })?;

                    csv_content = String::from_utf8(file_data.to_vec()).map_err(|e| {
                        RestError::BadRequest(format!("File is not valid UTF-8: {}", e))
                    })?;
                    break;
                }
            }

            if csv_content.is_empty() {
                return Err(RestError::BadRequest(
                    "No file provided or file is empty".to_string(),
                ));
            }

            // Import the CSV content
            let result: CsvImportResult = rest_state
                .csv_import_service()
                .import_products_csv(
                    &csv_content,
                    query.remove_unlisted,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await
                .map_err(RestError::from)?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(import_products_csv),
    components(schemas(CsvImportResult, CsvImportError, CsvFileUpload)),
    tags(
        (name = "CSV Import", description = "CSV import endpoints")
    )
)]
pub struct CsvImportApiDoc;
