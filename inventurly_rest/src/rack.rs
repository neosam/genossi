use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::RackTO;
use inventurly_service::rack::RackService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_racks::<RestState>))
        .route("/{id}", get(get_rack::<RestState>))
        .route("/", post(create_rack::<RestState>))
        .route("/{id}", put(update_rack::<RestState>))
        .route("/{id}", delete(delete_rack::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Racks",
    path = "",
    responses(
        (status = 200, description = "Get all racks", body = [RackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_racks<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let racks: Arc<[RackTO]> = rest_state
                .rack_service()
                .get_all(context.auth, None)
                .await?
                .iter()
                .map(RackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Racks",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 200, description = "Get rack by ID", body = RackTO),
        (status = 404, description = "Rack not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            if let Some(rack) = rest_state
                .rack_service()
                .get_by_id(id, context.auth, None)
                .await?
            {
                let rack_to = RackTO::from(&rack);
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::new(serde_json::to_string(&rack_to).unwrap()))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(404)
                    .body(Body::from("Rack not found"))
                    .unwrap())
            }
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Racks",
    path = "",
    request_body = RackTO,
    responses(
        (status = 200, description = "Rack created", body = RackTO),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(rack_to): Json<RackTO>,
) -> Response {
    error_handler(
        (async {
            let rack = inventurly_service::rack::Rack::from(&rack_to);
            let created_rack = rest_state
                .rack_service()
                .create(&rack, context.auth, None)
                .await?;
            let created_rack_to = RackTO::from(&created_rack);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&created_rack_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Racks",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Rack ID")
    ),
    request_body = RackTO,
    responses(
        (status = 200, description = "Rack updated", body = RackTO),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Rack not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(mut rack_to): Json<RackTO>,
) -> Response {
    error_handler(
        (async {
            rack_to.id = Some(id);
            let rack = inventurly_service::rack::Rack::from(&rack_to);
            let updated_rack = rest_state
                .rack_service()
                .update(&rack, context.auth, None)
                .await?;
            let updated_rack_to = RackTO::from(&updated_rack);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&updated_rack_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Racks",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 204, description = "Rack deleted"),
        (status = 404, description = "Rack not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .rack_service()
                .delete(id, context.auth, None)
                .await?;
            Ok(Response::builder()
                .status(204)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all_racks, get_rack, create_rack, update_rack, delete_rack),
    components(schemas(RackTO))
)]
pub struct ApiDoc;