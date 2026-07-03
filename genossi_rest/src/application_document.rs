//! Phase 25 Wave 3 (Plan 25-04): REST layer for the single-slot Application
//! document (Original-Antrag). Three admin-only endpoints delegate to the
//! [`ApplicationDocumentService`] built in Wave 2:
//!
//! - `POST   /` — Multipart upload; creates OR replaces the single slot.
//! - `GET    /` — Downloads the file bytes; `?meta=1` returns metadata JSON only.
//! - `DELETE /` — Soft-deletes the DB row and best-effort removes the file.
//!
//! All permission enforcement lives in the Service layer (CR-02 ordering is
//! pinned there); the REST handlers just extract the `Authentication<Context>`
//! and delegate. Multipart body limit + MIME allow-list are reused from the
//! MemberDocument surface (single maintenance point).

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    response::Response,
    routing::{delete, get, post},
    Extension, Router,
};
use genossi_rest_types::ApplicationDocumentTO;
use genossi_service::application_document::{
    ApplicationDocumentService, UploadApplicationDocument,
};
use genossi_service::member_document::{allowed_extensions, lookup_allowed_mime};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

/// Multipart schema for the upload endpoint. Only the `file` field is
/// consumed; other fields are ignored.
#[derive(ToSchema)]
#[allow(dead_code)]
struct ApplicationDocumentUpload {
    /// The file bytes.
    #[schema(format = Binary)]
    file: String,
}

/// Body-Limit for uploads. Intentionally the same 50 MB value as the
/// MemberDocument surface — deliberate parity, not accidental duplication.
const APPLICATION_DOCUMENT_BODY_LIMIT: usize = 50 * 1024 * 1024;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            post(upload_application_document::<RestState>)
                .layer(DefaultBodyLimit::max(APPLICATION_DOCUMENT_BODY_LIMIT)),
        )
        .route("/", get(download_application_document::<RestState>))
        .route("/", delete(delete_application_document::<RestState>))
}

/// Query params on GET `/`. `?meta=1` toggles metadata-only JSON instead of
/// downloading the file bytes.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DownloadQuery {
    /// When set to `1`, return metadata (JSON) instead of the file bytes.
    #[serde(default)]
    meta: Option<String>,
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Application Documents",
    path = "",
    params(
        ("application_id" = Uuid, Path, description = "Application ID"),
    ),
    request_body(
        content_type = "multipart/form-data",
        content = ApplicationDocumentUpload,
        description = "The Original-Antrag file"
    ),
    responses(
        (status = 201, description = "Document uploaded", body = ApplicationDocumentTO),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Application not found"),
        (status = 415, description = "Unsupported file type", body = genossi_rest_types::UnsupportedFileTypeResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upload_application_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(application_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            let mut file_name: Option<String> = None;
            let mut file_data: Option<Vec<u8>> = None;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read multipart: {}", e)))?
            {
                let name = field.name().unwrap_or("").to_string();
                if name.as_str() == "file" {
                    file_name = field.file_name().map(|s| s.to_string());
                    // Client MIME is intentionally ignored; server derives from extension.
                    file_data = Some(
                        field
                            .bytes()
                            .await
                            .map_err(|e| {
                                RestError::BadRequest(format!("Failed to read file: {}", e))
                            })?
                            .to_vec(),
                    );
                }
            }

            let data =
                file_data.ok_or_else(|| RestError::BadRequest("file is required".to_string()))?;
            let fname = file_name.unwrap_or_else(|| "antrag".to_string());

            // Extract extension and validate against the reused allow-list.
            let extension = fname
                .rsplit('.')
                .next()
                .filter(|ext| *ext != fname.as_str())
                .unwrap_or("");
            let server_mime = lookup_allowed_mime(extension).ok_or_else(|| {
                let allowed = allowed_extensions();
                RestError::UnsupportedMediaType(
                    serde_json::json!({
                        "error": format!("File type '{}' is not allowed", extension),
                        "allowed_extensions": allowed,
                    })
                    .to_string(),
                )
            })?;

            let upload = UploadApplicationDocument {
                application_id,
                file_name: fname,
                mime_type: server_mime.to_string(),
                data,
            };

            let doc = rest_state
                .application_document_service()
                .upload(upload, crate::extract_auth_context(Some(context))?, None)
                .await?;

            let to = ApplicationDocumentTO::from(&doc);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Application Documents",
    path = "",
    params(
        ("application_id" = Uuid, Path, description = "Application ID"),
        DownloadQuery,
    ),
    responses(
        (status = 200, description = "Document file bytes OR metadata JSON when ?meta=1"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn download_application_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(application_id): Path<Uuid>,
    Query(query): Query<DownloadQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;

            // `?meta=1` — metadata-only JSON path (skip the bytes fetch).
            if query.meta.as_deref() == Some("1") {
                let opt = rest_state
                    .application_document_service()
                    .get(application_id, auth, None)
                    .await?;
                let doc = opt.ok_or(RestError::NotFound)?;
                let to = ApplicationDocumentTO::from(&doc);
                return Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::new(serde_json::to_string(&to)?))
                    .unwrap());
            }

            let (doc, bytes) = rest_state
                .application_document_service()
                .download(application_id, auth, None)
                .await?;

            let content_disposition =
                crate::http_util::content_disposition_attachment(&doc.file_name);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", doc.mime_type.as_ref())
                .header("Content-Disposition", &content_disposition)
                .body(Body::from(bytes))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Application Documents",
    path = "",
    params(
        ("application_id" = Uuid, Path, description = "Application ID"),
    ),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_application_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(application_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .application_document_service()
                .delete(application_id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        upload_application_document,
        download_application_document,
        delete_application_document,
    ),
    components(schemas(ApplicationDocumentTO, ApplicationDocumentUpload)),
    tags((name = "Application Documents", description = "Single-slot Original-Antrag file per Application"))
)]
pub struct ApiDoc;
