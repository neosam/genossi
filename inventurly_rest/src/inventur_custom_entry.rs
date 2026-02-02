use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::InventurCustomEntryTO;
use inventurly_service::inventur_custom_entry::InventurCustomEntryService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_custom_entries,
        get_custom_entry,
        get_custom_entries_by_inventur,
        get_custom_entries_by_ean_and_inventur,
        create_custom_entry,
        update_custom_entry,
        delete_custom_entry
    ),
    components(
        schemas(InventurCustomEntryTO)
    ),
    tags(
        (name = "InventurCustomEntry", description = "Custom inventory entry endpoints for unknown products")
    )
)]
pub struct ApiDoc;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_custom_entries::<RestState>))
        .route("/{id}", get(get_custom_entry::<RestState>))
        .route("/", post(create_custom_entry::<RestState>))
        .route("/{id}", put(update_custom_entry::<RestState>))
        .route("/{id}", delete(delete_custom_entry::<RestState>))
        .route(
            "/by-inventur/{inventur_id}",
            get(get_custom_entries_by_inventur::<RestState>),
        )
        .route(
            "/by-ean/{ean}/inventur/{inventur_id}",
            get(get_custom_entries_by_ean_and_inventur::<RestState>),
        )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurCustomEntry",
    path = "",
    responses(
        (status = 200, description = "Get all custom entries", body = [InventurCustomEntryTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_custom_entries<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let entries: Arc<[InventurCustomEntryTO]> = rest_state
                .inventur_custom_entry_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(InventurCustomEntryTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&entries).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurCustomEntry",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Custom entry ID"),
    ),
    responses(
        (status = 200, description = "Custom entry found", body = InventurCustomEntryTO),
        (status = 404, description = "Custom entry not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_custom_entry<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let entry = rest_state
                .inventur_custom_entry_service()
                .get_by_id(id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let entry_to = InventurCustomEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&entry_to).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurCustomEntry",
    path = "/by-inventur/{inventur_id}",
    params(
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Custom entries for inventur", body = [InventurCustomEntryTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_custom_entries_by_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(inventur_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let entries: Arc<[InventurCustomEntryTO]> = rest_state
                .inventur_custom_entry_service()
                .get_by_inventur_id(
                    inventur_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(InventurCustomEntryTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&entries).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "InventurCustomEntry",
    path = "/by-ean/{ean}/inventur/{inventur_id}",
    params(
        ("ean" = String, Path, description = "Product EAN"),
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Custom entries for EAN and inventur", body = [InventurCustomEntryTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_custom_entries_by_ean_and_inventur<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((ean, inventur_id)): Path<(String, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let entries: Arc<[InventurCustomEntryTO]> = rest_state
                .inventur_custom_entry_service()
                .get_by_ean_and_inventur_id(
                    &ean,
                    inventur_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(InventurCustomEntryTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&entries).unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "InventurCustomEntry",
    path = "",
    request_body = InventurCustomEntryTO,
    responses(
        (status = 201, description = "Custom entry created successfully", body = InventurCustomEntryTO),
        (status = 400, description = "Invalid request body or validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_custom_entry<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(entry_to): Json<InventurCustomEntryTO>,
) -> Response {
    error_handler(
        (async {
            let entry =
                inventurly_service::inventur_custom_entry::InventurCustomEntry::from(&entry_to);
            let created = rest_state
                .inventur_custom_entry_service()
                .create(
                    &entry,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let created_to = InventurCustomEntryTO::from(&created);
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
    tag = "InventurCustomEntry",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Custom entry ID"),
    ),
    request_body = InventurCustomEntryTO,
    responses(
        (status = 200, description = "Custom entry updated successfully", body = InventurCustomEntryTO),
        (status = 404, description = "Custom entry not found"),
        (status = 400, description = "Invalid request body or validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_custom_entry<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(mut entry_to): Json<InventurCustomEntryTO>,
) -> Response {
    error_handler(
        (async {
            entry_to.id = Some(id);
            let entry =
                inventurly_service::inventur_custom_entry::InventurCustomEntry::from(&entry_to);
            let updated = rest_state
                .inventur_custom_entry_service()
                .update(
                    &entry,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let updated_to = InventurCustomEntryTO::from(&updated);
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
    tag = "InventurCustomEntry",
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "Custom entry ID"),
    ),
    responses(
        (status = 204, description = "Custom entry deleted successfully"),
        (status = 404, description = "Custom entry not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_custom_entry<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .inventur_custom_entry_service()
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
