use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    response::Response,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use genossi_rest_types::{MemberImportResultTO, MemberTO};
use genossi_service::member::MemberService;
use genossi_service::member_import::MemberImportService;
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

/// Multipart file upload schema for Swagger UI
#[derive(ToSchema)]
#[allow(dead_code)]
struct MemberImportUpload {
    /// Excel (.xlsx) file with member data
    #[schema(format = Binary)]
    file: String,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        .route("/{id}", get(get_member::<RestState>))
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
        .route("/import", post(import_members::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "",
    responses(
        (status = 200, description = "Get all members", body = [MemberTO]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_members<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let members: Arc<[MemberTO]> = rest_state
                .member_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(MemberTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&members).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Get member by ID", body = MemberTO),
        (status = 404, description = "Member not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_member<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let member = MemberTO::from(
                &rest_state
                    .member_service()
                    .get(member_id, crate::extract_auth_context(Some(context))?, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&member).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "",
    request_body = MemberTO,
    responses(
        (status = 200, description = "Create member", body = MemberTO),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_member<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(member): Json<MemberTO>,
) -> Response {
    error_handler(
        (async {
            let member = MemberTO::from(
                &rest_state
                    .member_service()
                    .create(
                        &(&member).into(),
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&member).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Members",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "Member ID")),
    request_body = MemberTO,
    responses(
        (status = 200, description = "Update member", body = MemberTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Member not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_member<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(mut member): Json<MemberTO>,
) -> Response {
    member.id = Some(member_id);
    error_handler(
        (async {
            let member = MemberTO::from(
                &rest_state
                    .member_service()
                    .update(
                        &(&member).into(),
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&member).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Members",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "Member ID")),
    responses(
        (status = 204, description = "Member deleted"),
        (status = 404, description = "Member not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_member<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .member_service()
                .delete(
                    member_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/import",
    request_body(content_type = "multipart/form-data", content = MemberImportUpload, description = "Excel (.xlsx) file with member data"),
    responses(
        (status = 200, description = "Import result", body = MemberImportResultTO),
        (status = 400, description = "Validation error (e.g. missing columns)"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn import_members<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            let field = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read multipart: {}", e)))?
                .ok_or_else(|| RestError::BadRequest("No file provided".to_string()))?;

            tracing::info!(
                "Import field: name={:?}, file_name={:?}, content_type={:?}",
                field.name().map(|s| s.to_string()),
                field.file_name().map(|s| s.to_string()),
                field.content_type().map(|s| s.to_string()),
            );

            let data = field
                .bytes()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read file: {}", e)))?;

            tracing::info!("Import file size: {} bytes", data.len());

            let result = rest_state
                .member_import_service()
                .import_members(&data[..], crate::extract_auth_context(Some(context))?)
                .await?;

            let result_to = MemberImportResultTO::from(result);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_members,
        get_member,
        create_member,
        update_member,
        delete_member,
        import_members
    ),
    components(schemas(MemberTO, MemberImportResultTO, genossi_rest_types::MemberImportErrorTO, MemberImportUpload)),
    tags((name = "Members", description = "Member management endpoints"))
)]
pub struct ApiDoc;
