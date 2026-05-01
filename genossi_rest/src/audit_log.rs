use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
    Extension, Router,
};
use genossi_dao::audit_log::{AuditLogDao, AuditQueryFilter};
use genossi_rest_types::{AuditLogEntryTO, BrokenLinkTO, PagedAuditLogTO, VerifyResponseTO};
use genossi_service::permission::{Authentication, PermissionService};
use serde::Deserialize;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

const DEFAULT_PAGE_SIZE: i64 = 50;
const ALLOWED_PAGE_SIZES: [i64; 5] = [25, 50, 100, 200, 500];

fn clamp_page_size(requested: Option<i64>) -> i64 {
    match requested {
        Some(n) if ALLOWED_PAGE_SIZES.contains(&n) => n,
        _ => DEFAULT_PAGE_SIZE,
    }
}

fn clamp_page(requested: Option<i64>) -> i64 {
    requested.map(|p| p.max(0)).unwrap_or(0)
}

pub trait AuditRestState: RestStateDef {
    type AuditLogDao: AuditLogDao + Send + Sync + 'static;

    fn audit_log_dao(&self) -> std::sync::Arc<Self::AuditLogDao>;
    fn audit_transaction(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        <Self::AuditLogDao as AuditLogDao>::Transaction,
                        genossi_dao::DaoError,
                    >,
                > + Send
                + '_,
        >,
    >;
}

pub fn generate_route<RestState: AuditRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_audit_log::<RestState>))
        .route("/verify", get(verify_chain::<RestState>))
        .route(
            "/{entity_type}/{entity_id}",
            get(get_audit_by_entity::<RestState>),
        )
}

#[derive(OpenApi)]
#[openapi(
    paths(get_audit_log, get_audit_by_entity, verify_chain),
    components(schemas(AuditLogEntryTO, PagedAuditLogTO, VerifyResponseTO, BrokenLinkTO)),
    tags((name = "Audit Log", description = "Audit log and integrity verification"))
)]
pub struct ApiDoc;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AuditQueryParams {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub action: Option<String>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Log",
    path = "",
    params(AuditQueryParams),
    responses(
        (status = 200, description = "Paginated audit log entries", body = PagedAuditLogTO),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_audit_log<RestState: AuditRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(params): Query<AuditQueryParams>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            let page = clamp_page(params.page);
            let size = clamp_page_size(params.size);
            let offset = page.saturating_mul(size);

            let filter = AuditQueryFilter {
                entity_type: params.entity_type,
                entity_id: params.entity_id,
                user_id: params.user_id,
                action: params.action,
                from: params.from,
                to: params.to,
            };

            let tx_count = rest_state
                .audit_transaction()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;
            let total = rest_state
                .audit_log_dao()
                .count(filter.clone(), tx_count)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let tx_query = rest_state
                .audit_transaction()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;
            let rows = rest_state
                .audit_log_dao()
                .query(filter, size, offset, tx_query)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let entries: Vec<AuditLogEntryTO> = rows.iter().map(AuditLogEntryTO::from).collect();
            let envelope = PagedAuditLogTO {
                entries,
                total,
                page,
                size,
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&envelope)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Log",
    path = "/{entity_type}/{entity_id}",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("entity_id" = Uuid, Path, description = "Entity UUID"),
    ),
    responses(
        (status = 200, description = "Audit log entries for entity", body = Vec<AuditLogEntryTO>),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_audit_by_entity<RestState: AuditRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((entity_type, entity_id)): Path<(String, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            let tx = rest_state
                .audit_transaction()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let entries = rest_state
                .audit_log_dao()
                .get_by_entity(&entity_type, entity_id, tx)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let result: Vec<AuditLogEntryTO> = entries.iter().map(AuditLogEntryTO::from).collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Log",
    path = "/verify",
    responses(
        (status = 200, description = "Hash chain verification result", body = VerifyResponseTO),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn verify_chain<RestState: AuditRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            let tx = rest_state
                .audit_transaction()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let entries = rest_state
                .audit_log_dao()
                .get_all_ordered(tx)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let broken = genossi_service_impl::audit_log::verify_chain(&entries);
            let total_entries = entries.len();

            let result = VerifyResponseTO {
                valid: broken.is_empty(),
                total_entries,
                broken_links: broken
                    .into_iter()
                    .map(|b| BrokenLinkTO {
                        entry_id: b.entry_id,
                        expected_hash: b.expected_hash,
                        actual_hash: b.actual_hash,
                    })
                    .collect(),
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result)?))
                .unwrap())
        })
        .await,
    )
}
