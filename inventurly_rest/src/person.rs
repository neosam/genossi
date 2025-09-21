use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::{delete, get, post, put};
use axum::{extract::State, response::Response};
use axum::{Extension, Json, Router};
use inventurly_rest_types::PersonTO;
use inventurly_service::person::PersonService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_persons::<RestState>))
        .route("/{id}", get(get_person::<RestState>))
        .route("/", post(create_person::<RestState>))
        .route("/{id}", put(update_person::<RestState>))
        .route("/{id}", delete(delete_person::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Persons",
    path = "",
    responses(
        (status = 200, description = "Get all persons", body = [PersonTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_persons<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let persons: Arc<[PersonTO]> = rest_state
                .person_service()
                .get_all(context.auth, None)
                .await?
                .iter()
                .map(PersonTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&persons).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Persons",
    params(
        ("id", description = "Person ID", example = "123e4567-e89b-12d3-a456-426614174000"),
    ),
    responses(
        (status = 200, description = "Get person by ID", body = PersonTO),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_person<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(person_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let person = PersonTO::from(
                &rest_state
                    .person_service()
                    .get(person_id, context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&person).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    path = "",
    tag = "Persons",
    request_body = PersonTO,
    responses(
        (status = 200, description = "Create person", body = PersonTO),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_person<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(person): Json<PersonTO>,
) -> Response {
    error_handler(
        (async {
            let person = PersonTO::from(
                &rest_state
                    .person_service()
                    .create(&(&person).into(), context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&person).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "Persons",
    params(
        ("id", description = "Person ID", example = "123e4567-e89b-12d3-a456-426614174000"),
    ),
    request_body = PersonTO,
    responses(
        (status = 200, description = "Update person", body = PersonTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_person<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(person_id): Path<Uuid>,
    Json(mut person): Json<PersonTO>,
) -> Response {
    person.id = Some(person_id);
    error_handler(
        (async {
            let person = PersonTO::from(
                &rest_state
                    .person_service()
                    .update(&(&person).into(), context.auth, None)
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&person).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Persons",
    params(
        ("id", description = "Person ID", example = "123e4567-e89b-12d3-a456-426614174000"),
    ),
    responses(
        (status = 204, description = "Person deleted successfully"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_person<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(person_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .person_service()
                .delete(person_id, context.auth, None)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_persons,
        get_person,
        create_person,
        update_person,
        delete_person
    ),
    components(
        schemas(PersonTO)
    ),
    tags(
        (name = "Persons", description = "Person management endpoints")
    )
)]
pub struct ApiDoc;
