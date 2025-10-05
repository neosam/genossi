use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::extract::Query;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use inventurly_rest_types::ProductTO;
use inventurly_service::product::ProductService;
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestStateDef};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_products::<RestState>))
        .route("/search", get(search_products::<RestState>))
        .route("/{ean}", get(get_product::<RestState>))
        .route("/", post(create_product::<RestState>))
        .route("/{ean}", put(update_product::<RestState>))
        .route("/{ean}", delete(delete_product::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Products",
    path = "",
    responses(
        (status = 200, description = "Get all products", body = [ProductTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_products<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let products: Arc<[ProductTO]> = rest_state
                .product_service()
                .get_all(context.auth, None)
                .await?
                .iter()
                .map(ProductTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&products).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Products",
    path = "/search",
    params(
        ("q" = String, Query, description = "Search query for product name or EAN", example = "apple"),
        ("limit" = Option<usize>, Query, description = "Maximum number of results", example = 20),
    ),
    responses(
        (status = 200, description = "Search results", body = [ProductTO]),
        (status = 400, description = "Bad request - missing or invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn search_products<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<SearchQuery>,
) -> Response {
    error_handler(
        (async {
            if query.q.trim().is_empty() {
                return Ok(Response::builder()
                    .status(400)
                    .header("Content-Type", "application/json")
                    .body(Body::new(r#"{"error": "Query parameter 'q' cannot be empty"}"#.to_string()))
                    .unwrap());
            }

            let products: Arc<[ProductTO]> = rest_state
                .product_service()
                .search(&query.q, query.limit, context.auth, None)
                .await?
                .iter()
                .map(ProductTO::from)
                .collect();
                
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&products).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    path = "/{ean}",
    tag = "Products",
    params(
        ("ean", description = "Product EAN", example = "4260474470041"),
    ),
    responses(
        (status = 200, description = "Get product by EAN", body = ProductTO),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
) -> Response {
    error_handler(
        (async {
            let product = ProductTO::from(
                &rest_state
                    .product_service()
                    .get_by_ean(&ean, context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    path = "",
    tag = "Products",
    request_body = ProductTO,
    responses(
        (status = 200, description = "Create product", body = ProductTO),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(product): Json<ProductTO>,
) -> Response {
    error_handler(
        (async {
            let product = ProductTO::from(
                &rest_state
                    .product_service()
                    .create(&(&product).into(), context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&product).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    path = "/{ean}",
    tag = "Products",
    params(
        ("ean", description = "Product EAN", example = "4260474470041"),
    ),
    request_body = ProductTO,
    responses(
        (status = 200, description = "Update product", body = ProductTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
    Json(mut product): Json<ProductTO>,
) -> Response {
    error_handler(
        (async {
            // First, get the existing product to get its ID
            let existing = rest_state
                .product_service()
                .get_by_ean(&ean, context.auth.clone(), None)
                .await?;
            
            // Use the ID from the existing product
            product.id = Some(existing.id);
            
            let updated = ProductTO::from(
                &rest_state
                    .product_service()
                    .update(&(&product).into(), context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&updated).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    path = "/{ean}",
    tag = "Products",
    params(
        ("ean", description = "Product EAN", example = "4260474470041"),
    ),
    responses(
        (status = 204, description = "Product deleted successfully"),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
) -> Response {
    error_handler(
        (async {
            // First, get the product to get its ID
            let product = rest_state
                .product_service()
                .get_by_ean(&ean, context.auth.clone(), None)
                .await?;
            
            rest_state
                .product_service()
                .delete(product.id, context.auth, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_products,
        search_products,
        get_product,
        create_product,
        update_product,
        delete_product
    ),
    components(
        schemas(ProductTO, inventurly_rest_types::Price)
    ),
    tags(
        (name = "Products", description = "Product management endpoints")
    )
)]
pub struct ApiDoc;