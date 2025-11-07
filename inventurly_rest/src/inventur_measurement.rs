use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::InventurMeasurementTO;
use inventurly_service::inventur_measurement::InventurMeasurementService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_measurements,
        get_measurement,
        get_measurements_by_inventur,
        get_measurements_by_product_and_inventur,
        create_measurement,
        update_measurement,
        delete_measurement
    ),
    components(
        schemas(InventurMeasurementTO)
    ),
    tags(
        (name = "InventurMeasurement", description = "Inventory measurement recording endpoints")
    )
)]
pub struct ApiDoc;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_measurements::<RestState>))
        .route("/{id}", get(get_measurement::<RestState>))
        .route("/", post(create_measurement::<RestState>))
        .route("/{id}", put(update_measurement::<RestState>))
        .route("/{id}", delete(delete_measurement::<RestState>))
        .route(
            "/by-inventur/{inventur_id}",
            get(get_measurements_by_inventur::<RestState>),
        )
        .route(
            "/by-product/{product_id}/inventur/{inventur_id}",
            get(get_measurements_by_product_and_inventur::<RestState>),
        )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurMeasurement",
    path = "",
    responses(
        (status = 200, description = "Get all measurements", body = [InventurMeasurementTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_measurements<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let measurements: Arc<[InventurMeasurementTO]> = rest_state
                .inventur_measurement_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(InventurMeasurementTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&measurements).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurMeasurement",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Measurement ID"),
    ),
    responses(
        (status = 200, description = "Measurement found", body = InventurMeasurementTO),
        (status = 404, description = "Measurement not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_measurement<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let measurement = rest_state
                .inventur_measurement_service()
                .get_by_id(id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let measurement_to = InventurMeasurementTO::from(&measurement);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&measurement_to).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurMeasurement",
    path = "/by-inventur/{inventur_id}",
    params(
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Measurements for inventur", body = [InventurMeasurementTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_measurements_by_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(inventur_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let measurements: Arc<[InventurMeasurementTO]> = rest_state
                .inventur_measurement_service()
                .get_by_inventur_id(
                    inventur_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(InventurMeasurementTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&measurements).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurMeasurement",
    path = "/by-product/{product_id}/inventur/{inventur_id}",
    params(
        ("product_id" = Uuid, Path, description = "Product ID"),
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Measurements for product in inventur", body = [InventurMeasurementTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_measurements_by_product_and_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((product_id, inventur_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let measurements: Arc<[InventurMeasurementTO]> = rest_state
                .inventur_measurement_service()
                .get_by_product_and_inventur(
                    product_id,
                    inventur_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(InventurMeasurementTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&measurements).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "InventurMeasurement",
    path = "",
    request_body = InventurMeasurementTO,
    responses(
        (status = 201, description = "Measurement created successfully", body = InventurMeasurementTO),
        (status = 400, description = "Invalid request body or validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_measurement<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(measurement_to): Json<InventurMeasurementTO>,
) -> Response {
    error_handler(
        (async {
            let measurement =
                inventurly_service::inventur_measurement::InventurMeasurement::from(&measurement_to);
            let created = rest_state
                .inventur_measurement_service()
                .create(
                    &measurement,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let created_to = InventurMeasurementTO::from(&created);
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
    tag = "InventurMeasurement",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Measurement ID"),
    ),
    request_body = InventurMeasurementTO,
    responses(
        (status = 200, description = "Measurement updated successfully", body = InventurMeasurementTO),
        (status = 404, description = "Measurement not found"),
        (status = 400, description = "Invalid request body or validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_measurement<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(mut measurement_to): Json<InventurMeasurementTO>,
) -> Response {
    error_handler(
        (async {
            measurement_to.id = Some(id);
            let measurement =
                inventurly_service::inventur_measurement::InventurMeasurement::from(&measurement_to);
            let updated = rest_state
                .inventur_measurement_service()
                .update(
                    &measurement,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let updated_to = InventurMeasurementTO::from(&updated);
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
    tag = "InventurMeasurement",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Measurement ID"),
    ),
    responses(
        (status = 204, description = "Measurement deleted successfully"),
        (status = 404, description = "Measurement not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_measurement<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .inventur_measurement_service()
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
