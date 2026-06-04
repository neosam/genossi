//! REST-Handler fuer v1.2 Phase 15 Membership-Adjustments.
//!
//! Exposed Endpoints (mounted via `member::generate_route`):
//! - `POST /api/members/{id}/cancel` -> `cancel_membership`
//! - `POST /api/members/{id}/increase-shares` -> `increase_shares`
//!
//! Pattern-Vorbild: `genossi_rest/src/member_action.rs::create_member_action`
//! (POST mit `Path<Uuid>` + `Json<RequestTO>` -> tuple-response via
//! `error_handler` + `serde_json::to_string`).
//!
//! BLOCKER 5 / D-15-12-Resolution: `ServiceError::PermissionDenied` wird per
//! globalem `From<ServiceError> for RestError`-Mapping
//! (`genossi_rest/src/lib.rs:115`) zu HTTP 401 (NICHT 403). OpenAPI-Annotation
//! listet 401, kein 403.

use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    Extension, Json,
};
use genossi_rest_types::{
    CancelMembershipRequestTO, IncreaseSharesRequestTO, MemberActionTO, MemberTO,
    MembershipAdjustResponseTO,
};
use genossi_service::membership_adjust::MembershipAdjustService;
use tracing::instrument;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/{id}/cancel",
    params(("id" = Uuid, Path, description = "Member ID")),
    request_body = CancelMembershipRequestTO,
    responses(
        (status = 200, description = "Cancellation successful", body = MembershipAdjustResponseTO),
        (status = 400, description = "Validation error (date bounds, etc.)"),
        // Pitfall 4 (Phase 14 RESEARCH) / D-15-12-Resolution:
        // ServiceError::PermissionDenied -> RestError::Unauthorized -> 401
        // per globalem Mapping in genossi_rest/src/lib.rs:115. KEIN 403.
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Member already cancelled"),
    ),
)]
pub async fn cancel_membership<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(req): Json<CancelMembershipRequestTO>,
) -> Response {
    error_handler(
        (async {
            let (action, member) = rest_state
                .membership_adjust_service()
                .cancel_membership(
                    member_id,
                    req.willensbekundung_date,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let response = MembershipAdjustResponseTO {
                action: MemberActionTO::from(&action),
                member: MemberTO::from(&member),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/{id}/increase-shares",
    params(("id" = Uuid, Path, description = "Member ID")),
    request_body = IncreaseSharesRequestTO,
    responses(
        (status = 200, description = "Increase successful", body = MembershipAdjustResponseTO),
        (status = 400, description = "Validation error (date bounds, shares <= 0, cancelled member)"),
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 404, description = "Member not found"),
    ),
)]
pub async fn increase_shares<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(req): Json<IncreaseSharesRequestTO>,
) -> Response {
    error_handler(
        (async {
            let (action, member) = rest_state
                .membership_adjust_service()
                .increase_shares(
                    member_id,
                    req.shares,
                    req.willensbekundung_date,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let response = MembershipAdjustResponseTO {
                action: MemberActionTO::from(&action),
                member: MemberTO::from(&member),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(cancel_membership, increase_shares),
    components(schemas(
        CancelMembershipRequestTO,
        IncreaseSharesRequestTO,
        MembershipAdjustResponseTO
    )),
    tags((name = "Members", description = "Phase 15 v1.2 membership-adjust endpoints"))
)]
pub struct ApiDoc;
