use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::{
    AddContainerToRackRequestTO, ContainerRackTO, ReorderContainersInRackRequestTO,
    SetContainerPositionRequestTO,
};
use inventurly_service::container_rack::ContainerRackService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", post(add_container_to_rack::<RestState>))
        .route("/reorder", put(reorder_containers_in_rack::<RestState>))
        .route("/position", put(set_container_position::<RestState>))
        .route(
            "/{container_id}/{rack_id}",
            delete(remove_container_from_rack::<RestState>),
        )
        .route(
            "/{container_id}/{rack_id}",
            get(get_container_rack_relationship::<RestState>),
        )
        .route(
            "/container/{container_id}",
            get(get_racks_for_container::<RestState>),
        )
        .route("/rack/{rack_id}", get(get_containers_in_rack::<RestState>))
        .route("/all", get(get_all_relationships::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Container-Rack",
    path = "",
    request_body = AddContainerToRackRequestTO,
    responses(
        (status = 200, description = "Container added to rack", body = ContainerRackTO),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn add_container_to_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(request): Json<AddContainerToRackRequestTO>,
) -> Response {
    error_handler(
        (async {
            let container_rack = rest_state
                .container_rack_service()
                .add_container_to_rack(request.container_id, request.rack_id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let container_rack_to = ContainerRackTO::from(&container_rack);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_rack_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Container-Rack",
    path = "/{container_id}/{rack_id}",
    params(
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 204, description = "Container removed from rack"),
        (status = 404, description = "Container-rack relationship not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn remove_container_from_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((container_id, rack_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .container_rack_service()
                .remove_container_from_rack(container_id, rack_id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Container-Rack",
    path = "/{container_id}/{rack_id}",
    params(
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 200, description = "Get container-rack relationship", body = ContainerRackTO),
        (status = 404, description = "Container-rack relationship not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_container_rack_relationship<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((container_id, rack_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            if let Some(container_rack) = rest_state
                .container_rack_service()
                .get_container_rack_relationship(container_id, rack_id, crate::extract_auth_context(Some(context))?, None)
                .await?
            {
                let container_rack_to = ContainerRackTO::from(&container_rack);
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::new(serde_json::to_string(&container_rack_to).unwrap()))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(404)
                    .body(Body::from("Container-rack relationship not found"))
                    .unwrap())
            }
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Container-Rack",
    path = "/container/{container_id}",
    params(
        ("container_id" = Uuid, Path, description = "Container ID")
    ),
    responses(
        (status = 200, description = "Get all racks for container", body = [ContainerRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_racks_for_container<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(container_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let container_racks: Arc<[ContainerRackTO]> = rest_state
                .container_rack_service()
                .get_racks_for_container(container_id, crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(ContainerRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Container-Rack",
    path = "/rack/{rack_id}",
    params(
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 200, description = "Get all containers in rack", body = [ContainerRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_containers_in_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(rack_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let container_racks: Arc<[ContainerRackTO]> = rest_state
                .container_rack_service()
                .get_containers_in_rack(rack_id, crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(ContainerRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Container-Rack",
    path = "/all",
    responses(
        (status = 200, description = "Get all container-rack relationships", body = [ContainerRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_relationships<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let container_racks: Arc<[ContainerRackTO]> = rest_state
                .container_rack_service()
                .get_all_relationships(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(ContainerRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Container-Rack",
    path = "/reorder",
    request_body = ReorderContainersInRackRequestTO,
    responses(
        (status = 200, description = "Containers reordered successfully", body = [ContainerRackTO]),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Rack or container not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn reorder_containers_in_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(request): Json<ReorderContainersInRackRequestTO>,
) -> Response {
    error_handler(
        (async {
            let container_racks: Arc<[ContainerRackTO]> = rest_state
                .container_rack_service()
                .reorder_containers_in_rack(
                    request.rack_id,
                    request.container_order,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(ContainerRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Container-Rack",
    path = "/position",
    request_body = SetContainerPositionRequestTO,
    responses(
        (status = 200, description = "Container position updated", body = ContainerRackTO),
        (status = 404, description = "Container-rack relationship not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn set_container_position<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(request): Json<SetContainerPositionRequestTO>,
) -> Response {
    error_handler(
        (async {
            let container_rack = rest_state
                .container_rack_service()
                .set_container_position_in_rack(
                    request.container_id,
                    request.rack_id,
                    request.position,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let container_rack_to = ContainerRackTO::from(&container_rack);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&container_rack_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        add_container_to_rack,
        remove_container_from_rack,
        get_container_rack_relationship,
        get_racks_for_container,
        get_containers_in_rack,
        get_all_relationships,
        reorder_containers_in_rack,
        set_container_position
    ),
    components(schemas(ContainerRackTO, AddContainerToRackRequestTO, ReorderContainersInRackRequestTO, SetContainerPositionRequestTO))
)]
pub struct ApiDoc;
