use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::extract::Query;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::ProductTO;
use inventurly_service::product::ProductService;
use inventurly_service::product_rack::ProductRackService;
use serde::Deserialize;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

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
        .route("/{id}", get(get_product::<RestState>))
        .route("/", post(create_product::<RestState>))
        .route("/{id}", put(update_product::<RestState>))
        .route("/{id}", delete(delete_product::<RestState>))
        // EAN-based endpoints for backward compatibility
        .route("/by-ean/{ean}", get(get_product_by_ean::<RestState>))
        .route("/by-ean/{ean}", put(update_product_by_ean::<RestState>))
        .route("/by-ean/{ean}", delete(delete_product_by_ean::<RestState>))
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
            let auth = crate::extract_auth_context(Some(context))?;
            let service_products = rest_state
                .product_service()
                .get_all(auth.clone(), None)
                .await?;

            // Enrich products with rack counts
            let mut products: Vec<ProductTO> = Vec::with_capacity(service_products.len());
            for product in service_products.iter() {
                let mut product_to = ProductTO::from(product);
                let racks = rest_state
                    .product_rack_service()
                    .get_racks_for_product(product.id, auth.clone(), None)
                    .await?;
                product_to.rack_count = Some(racks.len() as i64);
                products.push(product_to);
            }

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
                    .body(Body::new(
                        r#"{"error": "Query parameter 'q' cannot be empty"}"#.to_string(),
                    ))
                    .unwrap());
            }

            let products: Arc<[ProductTO]> = rest_state
                .product_service()
                .search(&query.q, query.limit, crate::extract_auth_context(Some(context))?, None)
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
    path = "/{id}",
    tag = "Products",
    params(
        ("id", description = "Product ID"),
    ),
    responses(
        (status = 200, description = "Get product by ID", body = ProductTO),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_product<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let product = ProductTO::from(
                &rest_state
                    .product_service()
                    .get_by_id(id, crate::extract_auth_context(Some(context))?, None)
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
                    .create(&(&product).into(), crate::extract_auth_context(Some(context))?, None)
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
    path = "/{id}",
    tag = "Products",
    params(
        ("id", description = "Product ID"),
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
    Path(id): Path<Uuid>,
    Json(mut product): Json<ProductTO>,
) -> Response {
    error_handler(
        (async {
            // Ensure the ID in the path matches the product
            product.id = Some(id);

            let updated = ProductTO::from(
                &rest_state
                    .product_service()
                    .update(&(&product).into(), crate::extract_auth_context(Some(context))?, None)
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
    path = "/{id}",
    tag = "Products",
    params(
        ("id", description = "Product ID"),
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
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .product_service()
                .delete(id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

// EAN-based endpoints for backward compatibility
#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    path = "/by-ean/{ean}",
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
pub async fn get_product_by_ean<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
) -> Response {
    error_handler(
        (async {
            let product = ProductTO::from(
                &rest_state
                    .product_service()
                    .get_by_ean(&ean, crate::extract_auth_context(Some(context))?, None)
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
    path = "/by-ean/{ean}",
    tag = "Products",
    params(
        ("ean", description = "Product EAN", example = "4260474470041"),
    ),
    request_body = ProductTO,
    responses(
        (status = 200, description = "Update product by EAN", body = ProductTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_product_by_ean<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
    Json(mut product): Json<ProductTO>,
) -> Response {
    error_handler(
        (async {
            // First, get the existing product to get its ID
            let auth = crate::extract_auth_context(Some(context))?;
            let existing = rest_state
                .product_service()
                .get_by_ean(&ean, auth.clone(), None)
                .await?;

            // Use the ID from the existing product
            product.id = Some(existing.id);

            let updated = ProductTO::from(
                &rest_state
                    .product_service()
                    .update(&(&product).into(), auth, None)
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
    path = "/by-ean/{ean}",
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
pub async fn delete_product_by_ean<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
) -> Response {
    error_handler(
        (async {
            // First, get the product to get its ID
            let auth = crate::extract_auth_context(Some(context))?;
            let product = rest_state
                .product_service()
                .get_by_ean(&ean, auth.clone(), None)
                .await?;

            rest_state
                .product_service()
                .delete(product.id, auth, None)
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
        delete_product,
        get_product_by_ean,
        update_product_by_ean,
        delete_product_by_ean
    ),
    components(
        schemas(ProductTO, inventurly_rest_types::Price)
    ),
    tags(
        (name = "Products", description = "Product management endpoints")
    )
)]
pub struct ApiDoc;
