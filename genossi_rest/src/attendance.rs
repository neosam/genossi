//! REST handlers for the Attendance aggregate (Phase 3 Plan 06).
//!
//! Routes (D-21):
//!   * `GET    /api/attendance/{assembly_id}/members?q=...` — reduced member
//!     list (ATTN-01, ATTN-02, ATTN-06).
//!   * `PUT    /api/attendance/{assembly_id}/{member_id}`   — toggle-on
//!     (ATTN-03, idempotent).
//!   * `DELETE /api/attendance/{assembly_id}/{member_id}`   — toggle-off
//!     (ATTN-04, idempotent).
//!   * `GET    /api/assembly/{assembly_id}/stats`           — live counter
//!     (ASSY-04).
//!
//! Permission decisions live in `AttendanceServiceImpl::check_assembly_access`
//! (Plan 05). The handlers ONLY map HTTP-status-codes; they do NOT touch any
//! permission logic themselves.
//!
//! D-26 / RESEARCH §DECISION CONFLICT 1 — local `map_attendance_error`:
//! `ServiceError::PermissionDenied` is mapped to `RestError::Forbidden(403)`
//! for attendance endpoints (instead of the global `Unauthorized(401)`).
//! This keeps Phase 1+2 endpoints unchanged while letting the Phase-4
//! frontend distinguish "no access" (403) from "session invalid" (401).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::{get, put},
    Extension, Router,
};
use genossi_rest_types::{AttendanceMemberTO, AttendanceStatsTO};
use genossi_service::attendance::AttendanceService;
use genossi_service::ServiceError;
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait AttendanceRestState: Clone + Send + Sync + 'static {
    type AttendanceService: AttendanceService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;
    fn attendance_service(&self) -> Arc<Self::AttendanceService>;
}

/// CONFLICT 1 (RESEARCH.md §DECISION CONFLICTS): differential mapping —
/// `PermissionDenied` → `403 Forbidden` for attendance endpoints (D-26).
/// All other ServiceError-variants delegate to the global `From<ServiceError>`
/// in `genossi_rest/src/lib.rs` (which maps PermissionDenied → 401), so 401
/// stays the default for non-attendance endpoints.
fn map_attendance_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}

/// Query parameters for `GET /api/attendance/{aid}/members`.
///
/// Only the substring filter is exposed (ATTN-02). Pagination is intentionally
/// out-of-scope for Phase 3 — Genossenschaften are typically <500 members.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListMembersQuery {
    /// Substring filter on last_name, first_name, or member_number (ATTN-02).
    #[serde(default)]
    pub q: Option<String>,
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Attendance",
    path = "/members",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID"),
        ListMembersQuery,
    ),
    responses(
        (status = 200, description = "Reduced member list (ATTN-01, ATTN-06)", body = [AttendanceMemberTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (helper wrong assembly or not Open / non-admin)"),
        (status = 404, description = "Assembly not found"),
    ),
)]
pub async fn list_attendance_members<RestState: RestStateDef + AttendanceRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(assembly_id): Path<Uuid>,
    Query(query): Query<ListMembersQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let rows = rest_state
                .attendance_service()
                .list_members(assembly_id, query.q, auth)
                .await
                .map_err(map_attendance_error)?;
            let tos: Vec<AttendanceMemberTO> =
                rows.iter().map(AttendanceMemberTO::from).collect();
            let body = serde_json::to_string(&tos)
                .map_err(|e| RestError::InternalError(format!("serialize: {}", e)))?;
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Attendance",
    path = "/{member_id}",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID"),
        ("member_id"   = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Marked present (idempotent — ATTN-03)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Assembly or member not in snapshot"),
    ),
)]
pub async fn mark_attendance_present<RestState: RestStateDef + AttendanceRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, member_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .attendance_service()
                .mark_present(assembly_id, member_id, auth)
                .await
                .map_err(map_attendance_error)?;
            Ok(Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Attendance",
    path = "/{member_id}",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID"),
        ("member_id"   = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Marked absent (idempotent — ATTN-04, returns 200 even on no-op)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Assembly or member not in snapshot"),
    ),
)]
pub async fn mark_attendance_absent<RestState: RestStateDef + AttendanceRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, member_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .attendance_service()
                .mark_absent(assembly_id, member_id, auth)
                .await
                .map_err(map_attendance_error)?;
            Ok(Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Attendance",
    path = "/stats",
    params(("assembly_id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Live counter (ASSY-04)", body = AttendanceStatsTO),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Assembly not found"),
    ),
)]
pub async fn get_assembly_stats<RestState: RestStateDef + AttendanceRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(assembly_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let stats = rest_state
                .attendance_service()
                .stats(assembly_id, auth)
                .await
                .map_err(map_attendance_error)?;
            let to = AttendanceStatsTO::from(&stats);
            let body = serde_json::to_string(&to)
                .map_err(|e| RestError::InternalError(format!("serialize: {}", e)))?;
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap())
        })
        .await,
    )
}

/// Router for `/api/attendance/{assembly_id}/...` (D-21).
///
/// The stats endpoint lives under a different namespace
/// (`/api/assembly/{assembly_id}/stats`) and is wired separately via
/// [`generate_stats_route`].
pub fn generate_attendance_route<RestState: RestStateDef + AttendanceRestState>(
) -> Router<RestState> {
    Router::new()
        .route("/members", get(list_attendance_members::<RestState>))
        .route(
            "/{member_id}",
            put(mark_attendance_present::<RestState>)
                .delete(mark_attendance_absent::<RestState>),
        )
}

/// Router for `/api/assembly/{assembly_id}/stats` (D-21).
///
/// Lives under the assembly namespace because the live counter is
/// semantically an assembly aspect, even though the implementation is in
/// `AttendanceService` (D-23).
pub fn generate_stats_route<RestState: RestStateDef + AttendanceRestState>(
) -> Router<RestState> {
    Router::new().route("/stats", get(get_assembly_stats::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_attendance_members,
        mark_attendance_present,
        mark_attendance_absent,
        get_assembly_stats,
    ),
    components(schemas(AttendanceMemberTO, AttendanceStatsTO, ListMembersQuery)),
    tags((name = "Attendance", description = "GV attendance recording (helpers + Vorstand)"))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_attendance_error_permission_denied_returns_forbidden() {
        // D-26: PermissionDenied -> 403 Forbidden, NOT 401 Unauthorized.
        let mapped = map_attendance_error(ServiceError::PermissionDenied);
        assert!(
            matches!(mapped, RestError::Forbidden(_)),
            "PermissionDenied must map to Forbidden(403) for attendance endpoints"
        );
    }

    #[test]
    fn test_map_attendance_error_entity_not_found_delegates_to_global() {
        // Other variants must delegate to the global From<ServiceError>.
        let mapped = map_attendance_error(ServiceError::EntityNotFound(Uuid::nil()));
        assert!(
            matches!(mapped, RestError::NotFound),
            "EntityNotFound must map to RestError::NotFound via global From"
        );
    }

    #[test]
    fn test_list_members_query_with_q_serializes_via_serde_json() {
        // Use serde_json roundtrip — guarantees the field is named `q` and
        // is optional. axum's Query extractor uses serde_urlencoded on the
        // raw HTTP query string; that path is exercised end-to-end in the
        // E2E test `test_attendance_members_substring_search_filters_by_query_param`.
        let json = serde_json::json!({ "q": "Mueller" });
        let parsed: ListMembersQuery = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.q.as_deref(), Some("Mueller"));
    }

    #[test]
    fn test_list_members_query_without_q_defaults_to_none() {
        let json = serde_json::json!({});
        let parsed: ListMembersQuery = serde_json::from_value(json).unwrap();
        assert!(parsed.q.is_none());
    }
}
