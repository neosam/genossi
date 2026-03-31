use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use genossi_rest_types::{MemberActionTO, MigrationStatusTO};
use genossi_service::member_action::MemberActionService;
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_member_actions::<RestState>))
        .route("/", post(create_member_action::<RestState>))
        .route("/{action_id}", get(get_member_action::<RestState>))
        .route("/{action_id}", put(update_member_action::<RestState>))
        .route("/{action_id}", delete(delete_member_action::<RestState>))
        .route("/migration-status", get(get_migration_status::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Member Actions",
    path = "",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "List all actions for member", body = [MemberActionTO]),
        (status = 404, description = "Member not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_member_actions<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let actions: Arc<[MemberActionTO]> = rest_state
                .member_action_service()
                .get_by_member(
                    member_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(MemberActionTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&actions).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Member Actions",
    path = "/{action_id}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("action_id" = Uuid, Path, description = "Action ID"),
    ),
    responses(
        (status = 200, description = "Get action by ID", body = MemberActionTO),
        (status = 404, description = "Action not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_member_action<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((_member_id, action_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let action = MemberActionTO::from(
                &rest_state
                    .member_action_service()
                    .get(action_id, crate::extract_auth_context(Some(context))?, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&action).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Member Actions",
    path = "",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    request_body = MemberActionTO,
    responses(
        (status = 200, description = "Create action", body = MemberActionTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Member not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn create_member_action<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(mut action): Json<MemberActionTO>,
) -> Response {
    action.member_id = member_id;
    error_handler(
        (async {
            let action = MemberActionTO::from(
                &rest_state
                    .member_action_service()
                    .create(
                        &(&action).into(),
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&action).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Member Actions",
    path = "/{action_id}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("action_id" = Uuid, Path, description = "Action ID"),
    ),
    request_body = MemberActionTO,
    responses(
        (status = 200, description = "Update action", body = MemberActionTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Action not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn update_member_action<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, action_id)): Path<(Uuid, Uuid)>,
    Json(mut action): Json<MemberActionTO>,
) -> Response {
    action.id = Some(action_id);
    action.member_id = member_id;
    error_handler(
        (async {
            let action = MemberActionTO::from(
                &rest_state
                    .member_action_service()
                    .update(
                        &(&action).into(),
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&action).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Member Actions",
    path = "/{action_id}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("action_id" = Uuid, Path, description = "Action ID"),
    ),
    responses(
        (status = 204, description = "Action deleted"),
        (status = 404, description = "Action not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn delete_member_action<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((_member_id, action_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .member_action_service()
                .delete(
                    action_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Member Actions",
    path = "/migration-status",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "Migration status", body = MigrationStatusTO),
        (status = 404, description = "Member not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_migration_status<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let status = MigrationStatusTO::from(
                &rest_state
                    .member_action_service()
                    .migration_status(
                        member_id,
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&status).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_member_actions,
        get_member_action,
        create_member_action,
        update_member_action,
        delete_member_action,
        get_migration_status
    ),
    components(schemas(MemberActionTO, genossi_rest_types::ActionTypeTO, MigrationStatusTO)),
    tags((name = "Member Actions", description = "Member action management endpoints"))
)]
pub struct ApiDoc;
