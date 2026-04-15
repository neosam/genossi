use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
    Extension, Router,
};
use genossi_dao::audit_log::AuditLogDao;
use genossi_rest_types::{AuditLogEntryTO, BrokenLinkTO, VerifyResponseTO};
use genossi_service::permission::{Authentication, PermissionService};
use serde::Deserialize;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

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
    components(schemas(AuditLogEntryTO, VerifyResponseTO, BrokenLinkTO)),
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
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Log",
    path = "",
    params(AuditQueryParams),
    responses(
        (status = 200, description = "Audit log entries", body = Vec<AuditLogEntryTO>),
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

            let tx = rest_state
                .audit_transaction()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let all_entries = rest_state
                .audit_log_dao()
                .get_all_ordered(tx)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let mut entries: Vec<AuditLogEntryTO> =
                all_entries.iter().map(AuditLogEntryTO::from).collect();

            if let Some(ref et) = params.entity_type {
                entries.retain(|e| e.entity_type == *et);
            }
            if let Some(ref eid) = params.entity_id {
                entries.retain(|e| e.entity_id == *eid);
            }
            if let Some(ref uid) = params.user_id {
                entries.retain(|e| e.user_id == *uid);
            }
            if let Some(ref action) = params.action {
                entries.retain(|e| e.action == *action);
            }
            if let Some(ref from) = params.from {
                entries.retain(|e| e.timestamp.as_str() >= from.as_str());
            }
            if let Some(ref to) = params.to {
                entries.retain(|e| e.timestamp.as_str() <= to.as_str());
            }

            entries.reverse();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&entries).unwrap()))
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
                .body(Body::new(serde_json::to_string(&result).unwrap()))
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
                .body(Body::new(serde_json::to_string(&result).unwrap()))
                .unwrap())
        })
        .await,
    )
}
