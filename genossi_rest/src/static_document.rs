use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    response::Response,
    routing::{delete, get, post},
    Extension, Router,
};
use genossi_mail::static_document_service::{
    StaticDocumentError, StaticDocumentService, UploadStaticDocument,
};
use genossi_service::permission::{Authentication, PermissionService, ADMIN_PRIVILEGE};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StaticDocumentTO {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created: String,
}

impl From<&genossi_mail::dao::StaticDocument> for StaticDocumentTO {
    fn from(doc: &genossi_mail::dao::StaticDocument) -> Self {
        let created = doc
            .created
            .assume_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_default();
        Self {
            id: doc.id.to_string(),
            name: doc.name.to_string(),
            filename: doc.filename.to_string(),
            content_type: doc.content_type.to_string(),
            size_bytes: doc.size_bytes,
            created,
        }
    }
}

/// Multipart upload schema documentation helper
#[derive(ToSchema)]
#[allow(dead_code)]
struct StaticDocumentUpload {
    /// Human-readable name for this document (defaults to filename if omitted)
    name: Option<String>,
    /// The file to upload
    #[schema(format = Binary)]
    file: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_documents, upload_document, download_document, delete_document),
    components(schemas(StaticDocumentTO, StaticDocumentUpload))
)]
pub struct ApiDoc;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_documents::<RestState>))
        .route("/", post(upload_document::<RestState>))
        .route("/{document_id}", get(download_document::<RestState>))
        .route("/{document_id}", delete(delete_document::<RestState>))
}

fn map_err(e: StaticDocumentError) -> RestError {
    match e {
        StaticDocumentError::NotFound => RestError::NotFound,
        StaticDocumentError::Validation(msg) => RestError::BadRequest(msg.to_string()),
        StaticDocumentError::Storage(msg) => {
            RestError::InternalError(format!("Storage error: {}", msg))
        }
        StaticDocumentError::DataAccess(msg) => {
            RestError::InternalError(format!("Database error: {}", msg))
        }
    }
}

async fn require_admin<RestState: RestStateDef>(
    rest_state: &RestState,
    context: Context,
) -> Result<(), RestError> {
    let auth = crate::extract_auth_context(Some(context))?;
    let authentication: Authentication<_> = Authentication::from(auth);
    rest_state
        .permission_service()
        .check_permission(ADMIN_PRIVILEGE, authentication)
        .await
        .map_err(RestError::from)
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Static Documents",
    path = "",
    responses(
        (status = 200, description = "List all active static documents", body = [StaticDocumentTO]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_documents<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            require_admin(&*rest_state, context).await?;
            let docs = rest_state
                .static_document_service()
                .list()
                .await
                .map_err(map_err)?;
            let to: Vec<StaticDocumentTO> = docs.iter().map(StaticDocumentTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Static Documents",
    path = "",
    request_body(content_type = "multipart/form-data", content = StaticDocumentUpload, description = "Upload a static document"),
    responses(
        (status = 201, description = "Document uploaded", body = StaticDocumentTO),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upload_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            require_admin(&*rest_state, context).await?;

            let mut name: Option<String> = None;
            let mut filename: Option<String> = None;
            let mut content_type: Option<String> = None;
            let mut file_data: Option<Vec<u8>> = None;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read multipart: {}", e)))?
            {
                let field_name = field.name().unwrap_or("").to_string();
                match field_name.as_str() {
                    "name" => {
                        name = Some(
                            field
                                .text()
                                .await
                                .map_err(|e| RestError::BadRequest(e.to_string()))?,
                        );
                    }
                    "file" => {
                        filename = field.file_name().map(|s| s.to_string());
                        content_type = field.content_type().map(|s| s.to_string());
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
                    _ => {}
                }
            }

            let data = file_data
                .ok_or_else(|| RestError::BadRequest("file field is required".to_string()))?;
            let fname = filename.clone().unwrap_or_else(|| "document".to_string());
            let name_final = name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(fname.clone());
            let ctype = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

            let upload = UploadStaticDocument {
                name: name_final,
                filename: fname,
                content_type: ctype,
                data,
            };

            let doc = rest_state
                .static_document_service()
                .upload(upload)
                .await
                .map_err(map_err)?;
            let to = StaticDocumentTO::from(&doc);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Static Documents",
    path = "/{document_id}",
    params(
        ("document_id" = Uuid, Path, description = "Static document ID"),
    ),
    responses(
        (status = 200, description = "Document file", content_type = "application/octet-stream"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn download_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(document_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            require_admin(&*rest_state, context).await?;
            let (doc, data) = rest_state
                .static_document_service()
                .load_bytes(document_id)
                .await
                .map_err(map_err)?;
            let content_disposition = format!("attachment; filename=\"{}\"", doc.filename);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", doc.content_type.as_ref())
                .header("Content-Disposition", content_disposition)
                .body(Body::from(data))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Static Documents",
    path = "/{document_id}",
    params(
        ("document_id" = Uuid, Path, description = "Static document ID"),
    ),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(document_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            require_admin(&*rest_state, context).await?;
            rest_state
                .static_document_service()
                .delete(document_id)
                .await
                .map_err(map_err)?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}
