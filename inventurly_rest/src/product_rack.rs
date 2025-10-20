use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::{AddProductToRackRequestTO, ProductRackTO};
use inventurly_service::product_rack::ProductRackService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", post(add_product_to_rack::<RestState>))
        .route(
            "/{product_id}/{rack_id}",
            delete(remove_product_from_rack::<RestState>),
        )
        .route(
            "/{product_id}/{rack_id}",
            get(get_product_rack_relationship::<RestState>),
        )
        .route(
            "/product/{product_id}",
            get(get_racks_for_product::<RestState>),
        )
        .route("/rack/{rack_id}", get(get_products_in_rack::<RestState>))
        .route("/all", get(get_all_relationships::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Product-Rack",
    path = "",
    request_body = AddProductToRackRequestTO,
    responses(
        (status = 200, description = "Product added to rack", body = ProductRackTO),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn add_product_to_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(request): Json<AddProductToRackRequestTO>,
) -> Response {
    error_handler(
        (async {
            let product_rack = rest_state
                .product_rack_service()
                .add_product_to_rack(request.product_id, request.rack_id, context.auth, None)
                .await?;
            let product_rack_to = ProductRackTO::from(&product_rack);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product_rack_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Product-Rack",
    path = "/{product_id}/{rack_id}",
    params(
        ("product_id" = Uuid, Path, description = "Product ID"),
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 204, description = "Product removed from rack"),
        (status = 404, description = "Product-rack relationship not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn remove_product_from_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((product_id, rack_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .product_rack_service()
                .remove_product_from_rack(product_id, rack_id, context.auth, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Product-Rack",
    path = "/{product_id}/{rack_id}",
    params(
        ("product_id" = Uuid, Path, description = "Product ID"),
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 200, description = "Get product-rack relationship", body = ProductRackTO),
        (status = 404, description = "Product-rack relationship not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_product_rack_relationship<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((product_id, rack_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            if let Some(product_rack) = rest_state
                .product_rack_service()
                .get_product_rack_relationship(product_id, rack_id, context.auth, None)
                .await?
            {
                let product_rack_to = ProductRackTO::from(&product_rack);
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::new(serde_json::to_string(&product_rack_to).unwrap()))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(404)
                    .body(Body::from("Product-rack relationship not found"))
                    .unwrap())
            }
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Product-Rack",
    path = "/product/{product_id}",
    params(
        ("product_id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Get all racks for product", body = [ProductRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_racks_for_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(product_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let product_racks: Arc<[ProductRackTO]> = rest_state
                .product_rack_service()
                .get_racks_for_product(product_id, context.auth, None)
                .await?
                .iter()
                .map(ProductRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Product-Rack",
    path = "/rack/{rack_id}",
    params(
        ("rack_id" = Uuid, Path, description = "Rack ID")
    ),
    responses(
        (status = 200, description = "Get all products in rack", body = [ProductRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_products_in_rack<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(rack_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let product_racks: Arc<[ProductRackTO]> = rest_state
                .product_rack_service()
                .get_products_in_rack(rack_id, context.auth, None)
                .await?
                .iter()
                .map(ProductRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Product-Rack",
    path = "/all",
    responses(
        (status = 200, description = "Get all product-rack relationships", body = [ProductRackTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_relationships<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let product_racks: Arc<[ProductRackTO]> = rest_state
                .product_rack_service()
                .get_all_relationships(context.auth, None)
                .await?
                .iter()
                .map(ProductRackTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product_racks).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        add_product_to_rack,
        remove_product_from_rack,
        get_product_rack_relationship,
        get_racks_for_product,
        get_products_in_rack,
        get_all_relationships
    ),
    components(schemas(ProductRackTO, AddProductToRackRequestTO))
)]
pub struct ApiDoc;
