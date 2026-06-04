use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    response::Response,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use genossi_mail::service::MailService;
use genossi_rest_types::{MemberImportResultTO, MemberSlimTO, MemberTO};
use genossi_service::member::MemberService;
use genossi_service::member_import::MemberImportService;
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
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
    // Pitfall 1 (Phase 14 RESEARCH §"Sub-Route-Ordering"):
    // Literal sub-routes MUST be declared before any `/{id}` path-parameter
    // route, because axum matches routes in declaration order. If
    // `/transfer-recipients` is declared after `/{id}`, axum tries to parse the
    // literal "transfer-recipients" as a Uuid and fails with HTTP 400. The same
    // applies to `/import` and `/not-reached-by/{job_id}` (currently safe only
    // because their HTTP methods differ from the colliding `/{id}` route).
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        // Literal sub-routes FIRST — MUST be declared before /{id} (Pitfall 1).
        // axum does not parse "transfer-recipients" as a UUID when this rule holds.
        .route(
            "/transfer-recipients",
            get(get_transfer_recipients::<RestState>),
        )
        .route("/import", post(import_members::<RestState>))
        .route(
            "/not-reached-by/{job_id}",
            get(get_members_not_reached_by::<RestState>),
        )
        // Path-parameter routes LAST.
        .route("/{id}", get(get_member::<RestState>))
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
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
                .body(Body::new(serde_json::to_string(&members)?))
                .unwrap())
        })
        .await,
    )
}

/// Query parameters for `GET /api/members/transfer-recipients`.
///
/// `exclude_self` ist die UUID des aktuellen Mitglieds, das den Transfer-Dialog
/// öffnet — wird aus der Empfänger-Liste ausgefiltert (Self-Transfer-Block).
#[derive(Debug, Deserialize, IntoParams)]
pub struct TransferRecipientsQuery {
    /// UUID des aktuellen Mitglieds — wird aus der Ergebnis-Liste ausgefiltert.
    pub exclude_self: Uuid,
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "/transfer-recipients",
    params(TransferRecipientsQuery),
    responses(
        (status = 200, description = "Aktive Transfer-Empfaenger (ohne self)", body = [MemberSlimTO]),
        (status = 400, description = "Invalid exclude_self UUID format"),
        // Pitfall 4 (Phase 14 RESEARCH §"PermissionDenied -> 401"):
        // The global From<ServiceError> for RestError maps PermissionDenied to
        // Unauthorized (401), NOT Forbidden (403). Do NOT add a 403 entry.
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_transfer_recipients<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<TransferRecipientsQuery>,
) -> Response {
    error_handler(
        (async {
            let members: Vec<MemberSlimTO> = rest_state
                .member_service()
                .list_transfer_recipients(
                    query.exclude_self,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(MemberSlimTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&members)?))
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
                .body(Body::new(serde_json::to_string(&member)?))
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
                .body(Body::new(serde_json::to_string(&member)?))
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
                .body(Body::new(serde_json::to_string(&member)?))
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
                .delete(member_id, crate::extract_auth_context(Some(context))?, None)
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
                .body(Body::new(serde_json::to_string(&result_to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "/not-reached-by/{job_id}",
    params(
        ("job_id" = String, Path, description = "Mail job UUID"),
    ),
    responses(
        (status = 200, description = "Members not reached by the given mail job", body = [MemberTO]),
        (status = 404, description = "Mail job not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_members_not_reached_by<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(job_id): Path<String>,
) -> Response {
    error_handler(
        (async {
            let job_id = uuid::Uuid::parse_str(&job_id).map_err(|_| crate::RestError::NotFound)?;

            let reached_ids = rest_state
                .mail_service()
                .get_reached_member_ids(job_id)
                .await?;

            let all_members = rest_state
                .member_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?;

            let not_reached: Vec<MemberTO> = all_members
                .iter()
                .filter(|m| m.deleted.is_none())
                .filter(|m| !reached_ids.contains(&m.id))
                .map(MemberTO::from)
                .collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&not_reached)?))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_members,
        get_transfer_recipients,
        get_member,
        create_member,
        update_member,
        delete_member,
        import_members,
        get_members_not_reached_by
    ),
    components(schemas(MemberTO, MemberSlimTO, genossi_rest_types::SalutationTO, genossi_rest_types::MemberStatusTO, MemberImportResultTO, genossi_rest_types::MemberImportErrorTO, MemberImportUpload)),
    tags((name = "Members", description = "Member management endpoints"))
)]
pub struct ApiDoc;
