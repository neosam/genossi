use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::{ChangeInventurStatusRequestTO, InventurTO};
use inventurly_service::inventur::InventurService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_inventurs,
        get_inventur,
        create_inventur,
        update_inventur,
        change_inventur_status,
        delete_inventur
    ),
    components(
        schemas(InventurTO, ChangeInventurStatusRequestTO)
    ),
    tags(
        (name = "Inventur", description = "Inventory counting session management endpoints")
    )
)]
pub struct ApiDoc;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_inventurs::<RestState>))
        .route("/{id}", get(get_inventur::<RestState>))
        .route("/", post(create_inventur::<RestState>))
        .route("/{id}", put(update_inventur::<RestState>))
        .route("/{id}/status", put(change_inventur_status::<RestState>))
        .route("/{id}", delete(delete_inventur::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Inventur",
    path = "",
    responses(
        (status = 200, description = "Get all inventur sessions", body = [InventurTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_inventurs<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let inventurs: Arc<[InventurTO]> = rest_state
                .inventur_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(InventurTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&inventurs).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Inventur",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Inventur session found", body = InventurTO),
        (status = 404, description = "Inventur not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let inventur = rest_state
                .inventur_service()
                .get_by_id(id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let inventur_to = InventurTO::from(&inventur);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&inventur_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Inventur",
    path = "",
    request_body = InventurTO,
    responses(
        (status = 201, description = "Inventur created successfully", body = InventurTO),
        (status = 400, description = "Invalid request body"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(inventur_to): Json<InventurTO>,
) -> Response {
    error_handler(
        (async {
            let inventur = inventurly_service::inventur::Inventur::from(&inventur_to);
            let created = rest_state
                .inventur_service()
                .create(&inventur, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let created_to = InventurTO::from(&created);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&created_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Inventur",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Inventur ID"),
    ),
    request_body = InventurTO,
    responses(
        (status = 200, description = "Inventur updated successfully", body = InventurTO),
        (status = 404, description = "Inventur not found"),
        (status = 400, description = "Invalid request body"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(mut inventur_to): Json<InventurTO>,
) -> Response {
    error_handler(
        (async {
            inventur_to.id = Some(id);
            let inventur = inventurly_service::inventur::Inventur::from(&inventur_to);
            let updated = rest_state
                .inventur_service()
                .update(&inventur, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let updated_to = InventurTO::from(&updated);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&updated_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Inventur",
    path = "/{id}/status",
    params(
        ("id" = Uuid, Path, description = "Inventur ID"),
    ),
    request_body = ChangeInventurStatusRequestTO,
    responses(
        (status = 200, description = "Inventur status changed successfully", body = InventurTO),
        (status = 404, description = "Inventur not found"),
        (status = 400, description = "Invalid status transition"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn change_inventur_status<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(request): Json<ChangeInventurStatusRequestTO>,
) -> Response {
    error_handler(
        (async {
            let updated = rest_state
                .inventur_service()
                .change_status(
                    id,
                    &request.status,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let updated_to = InventurTO::from(&updated);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&updated_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Inventur",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 204, description = "Inventur deleted successfully"),
        (status = 404, description = "Inventur not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .inventur_service()
                .delete(id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            Ok(Response::builder()
                .status(204)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}
