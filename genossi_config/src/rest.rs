use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, put};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};

use crate::dao::ConfigEntry;
use crate::service::{ConfigService, ConfigServiceError};

pub trait ConfigRestState: Clone + Send + Sync + 'static {
    type ConfigService: ConfigService;
    fn config_service(&self) -> Arc<Self::ConfigService>;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfigEntryTO {
    #[schema(example = "smtp_host")]
    pub key: String,
    #[schema(example = "mail.example.com")]
    pub value: String,
    #[schema(example = "string")]
    pub value_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SetConfigRequest {
    #[schema(example = "mail.example.com")]
    pub value: String,
    #[schema(example = "string")]
    pub value_type: String,
}

impl From<&ConfigEntry> for ConfigEntryTO {
    fn from(entry: &ConfigEntry) -> Self {
        let value = if entry.value_type.as_ref() == "secret" {
            "***".to_string()
        } else {
            entry.value.to_string()
        };
        Self {
            key: entry.key.to_string(),
            value,
            value_type: entry.value_type.to_string(),
        }
    }
}

fn error_handler(result: Result<Response, ConfigServiceError>) -> Response {
    match result {
        Ok(response) => response,
        Err(ConfigServiceError::NotFound) => Response::builder()
            .status(404)
            .body(Body::from("Not found"))
            .unwrap(),
        Err(ConfigServiceError::ValidationError(msg)) => Response::builder()
            .status(400)
            .body(Body::from(msg.to_string()))
            .unwrap(),
        Err(ConfigServiceError::DataAccess(msg)) => {
            tracing::error!("Config data access error: {}", msg);
            Response::builder()
                .status(500)
                .body(Body::from("Internal server error"))
                .unwrap()
        }
    }
}

pub fn generate_route<S: ConfigRestState>() -> Router<S> {
    Router::new()
        .route("/", get(get_all::<S>))
        .route("/{key}", put(set_config::<S>))
        .route("/{key}", delete(delete_config::<S>))
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all, set_config, delete_config),
    components(schemas(ConfigEntryTO, SetConfigRequest)),
    tags((name = "Config", description = "Configuration management endpoints"))
)]
pub struct ApiDoc;

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Config",
    path = "",
    responses(
        (status = 200, description = "Get all config entries", body = [ConfigEntryTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all<S: ConfigRestState>(state: State<S>) -> Response {
    error_handler(
        (async {
            let entries: Vec<ConfigEntryTO> = state
                .config_service()
                .get_all()
                .await?
                .iter()
                .map(ConfigEntryTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&entries).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    put,
    tag = "Config",
    path = "/{key}",
    request_body = SetConfigRequest,
    params(
        ("key" = String, Path, description = "Config key")
    ),
    responses(
        (status = 200, description = "Config entry set", body = ConfigEntryTO),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn set_config<S: ConfigRestState>(
    state: State<S>,
    Path(key): Path<String>,
    axum::Json(body): axum::Json<SetConfigRequest>,
) -> Response {
    error_handler(
        (async {
            let entry = ConfigEntry {
                key: Arc::from(key.as_str()),
                value: Arc::from(body.value.as_str()),
                value_type: Arc::from(body.value_type.as_str()),
            };
            state.config_service().set(&entry).await?;
            let to = ConfigEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    delete,
    tag = "Config",
    path = "/{key}",
    params(
        ("key" = String, Path, description = "Config key")
    ),
    responses(
        (status = 204, description = "Config entry deleted"),
        (status = 404, description = "Config entry not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_config<S: ConfigRestState>(
    state: State<S>,
    Path(key): Path<String>,
) -> Response {
    error_handler(
        (async {
            state.config_service().delete(&key).await?;
            Ok(Response::builder()
                .status(204)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}
