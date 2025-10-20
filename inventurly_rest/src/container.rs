use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query};
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::ContainerTO;
use inventurly_service::container::{Container, ContainerService};
use serde::Deserialize;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_containers::<RestState>))
        .route("/{id}", get(get_container::<RestState>))
        .route("/", post(create_container::<RestState>))
        .route("/{id}", put(update_container::<RestState>))
        .route("/{id}", delete(delete_container::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_containers,
        get_container,
        create_container,
        update_container,
        delete_container
    ),
    components(schemas(ContainerTO))
)]
pub struct ApiDoc;

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Containers",
    path = "",
    responses(
        (status = 200, description = "Get all containers", body = [ContainerTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_containers<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(params): Query<SearchQuery>,
) -> Response {
    error_handler(
        (async {
            let containers = if let Some(query) = params.q {
                rest_state
                    .container_service()
                    .search(&query, params.limit, context.auth, None)
                    .await?
            } else {
                rest_state
                    .container_service()
                    .get_all(context.auth, None)
                    .await?
            };

            let containers_to: Arc<[ContainerTO]> =
                containers.iter().map(ContainerTO::from).collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&containers_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Containers",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Container ID")
    ),
    responses(
        (status = 200, description = "Get container by ID", body = ContainerTO),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_container<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let container = rest_state
                .container_service()
                .get_by_id(id, context.auth, None)
                .await?;

            let container_to = ContainerTO::from(&container);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Containers",
    path = "",
    request_body = ContainerTO,
    responses(
        (status = 201, description = "Create container", body = ContainerTO),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_container<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(container_to): Json<ContainerTO>,
) -> Response {
    error_handler(
        (async {
            let container = Container::from(&container_to);
            let created_container = rest_state
                .container_service()
                .create(&container, context.auth, None)
                .await?;

            let created_container_to = ContainerTO::from(&created_container);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&created_container_to).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Containers",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Container ID")
    ),
    request_body = ContainerTO,
    responses(
        (status = 200, description = "Update container", body = ContainerTO),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_container<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(mut container_to): Json<ContainerTO>,
) -> Response {
    error_handler(
        (async {
            // Ensure the ID matches the path parameter
            container_to.id = Some(id);

            let container = Container::from(&container_to);
            let updated_container = rest_state
                .container_service()
                .update(&container, context.auth, None)
                .await?;

            let updated_container_to = ContainerTO::from(&updated_container);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&updated_container_to).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Containers",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Container ID")
    ),
    responses(
        (status = 204, description = "Delete container"),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_container<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .container_service()
                .delete(id, context.auth, None)
                .await?;

            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}
