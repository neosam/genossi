use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use inventurly_rest_types::{
    CheckDuplicateRequestTO, DuplicateDetectionConfigTO, DuplicateDetectionResultTO,
    DuplicateMatchTO,
};
use inventurly_service::duplicate_detection::DuplicateDetectionService;
use inventurly_service::product::ProductService;
use serde::Deserialize;
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/products", get(find_all_duplicates::<RestState>))
        .route("/products/{ean}", get(find_duplicates_by_ean::<RestState>))
        .route("/check", post(check_potential_duplicate::<RestState>))
}

#[derive(Debug, Deserialize)]
pub struct DuplicateQueryParams {
    similarity_threshold: Option<f64>,
    exact_match_weight: Option<f64>,
    word_order_weight: Option<f64>,
    levenshtein_weight: Option<f64>,
    jaro_winkler_weight: Option<f64>,
    category_aware: Option<bool>,
}

impl From<DuplicateQueryParams> for DuplicateDetectionConfigTO {
    fn from(params: DuplicateQueryParams) -> Self {
        let default_config =
            inventurly_service::duplicate_detection::DuplicateDetectionConfig::default();
        Self {
            similarity_threshold: params
                .similarity_threshold
                .unwrap_or(default_config.similarity_threshold),
            exact_match_weight: params
                .exact_match_weight
                .unwrap_or(default_config.exact_match_weight),
            word_order_weight: params
                .word_order_weight
                .unwrap_or(default_config.word_order_weight),
            levenshtein_weight: params
                .levenshtein_weight
                .unwrap_or(default_config.levenshtein_weight),
            jaro_winkler_weight: params
                .jaro_winkler_weight
                .unwrap_or(default_config.jaro_winkler_weight),
            category_aware: params
                .category_aware
                .unwrap_or(default_config.category_aware),
        }
    }
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Duplicate Detection",
    path = "/products",
    params(
        ("similarity_threshold" = Option<f64>, Query, description = "Minimum similarity threshold (0.0-1.0). Default: 0.55. Lower values (0.45) catch more duplicates, higher values (0.7) are more conservative.", example = 0.55),
        ("exact_match_weight" = Option<f64>, Query, description = "Weight for exact matches (0.0-1.0). Reduced because exact matches are rare in real-world data.", example = 0.3),
        ("word_order_weight" = Option<f64>, Query, description = "Weight for word order similarity (0.0-1.0). Important for German text where word order varies.", example = 0.4),
        ("levenshtein_weight" = Option<f64>, Query, description = "Weight for Levenshtein similarity (0.0-1.0). Good for detecting typos like 'süß' vs 'süss'.", example = 0.2),
        ("jaro_winkler_weight" = Option<f64>, Query, description = "Weight for Jaro-Winkler similarity (0.0-1.0). Supplementary algorithm for character transpositions.", example = 0.1),
        ("category_aware" = Option<bool>, Query, description = "Enable category-aware matching. Considers sales_unit and requires_weighing fields.", example = true),
    ),
    responses(
        (status = 200, description = "All duplicate groups found", body = Vec<DuplicateDetectionResultTO>),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn find_all_duplicates<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(params): Query<DuplicateQueryParams>,
) -> Response {
    error_handler(
        (async {
            let config_to: DuplicateDetectionConfigTO = params.into();
            let config = config_to.into();

            let results = rest_state
                .duplicate_detection_service()
                .find_all_duplicates(Some(config), crate::extract_auth_context(Some(context))?, None)
                .await
                .map_err(RestError::from)?;

            let response_results: Vec<DuplicateDetectionResultTO> = results
                .into_iter()
                .map(DuplicateDetectionResultTO::from)
                .collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response_results).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Duplicate Detection",
    path = "/products/{ean}",
    params(
        ("ean" = String, Path, description = "Product EAN code", example = "4260474470041"),
        ("similarity_threshold" = Option<f64>, Query, description = "Minimum similarity threshold (0.0-1.0). Default: 0.55. Lower values (0.45) catch more duplicates, higher values (0.7) are more conservative.", example = 0.55),
        ("exact_match_weight" = Option<f64>, Query, description = "Weight for exact matches (0.0-1.0). Reduced because exact matches are rare in real-world data.", example = 0.3),
        ("word_order_weight" = Option<f64>, Query, description = "Weight for word order similarity (0.0-1.0). Important for German text where word order varies.", example = 0.4),
        ("levenshtein_weight" = Option<f64>, Query, description = "Weight for Levenshtein similarity (0.0-1.0). Good for detecting typos like 'süß' vs 'süss'.", example = 0.2),
        ("jaro_winkler_weight" = Option<f64>, Query, description = "Weight for Jaro-Winkler similarity (0.0-1.0). Supplementary algorithm for character transpositions.", example = 0.1),
        ("category_aware" = Option<bool>, Query, description = "Enable category-aware matching. Considers sales_unit and requires_weighing fields.", example = true),
    ),
    responses(
        (status = 200, description = "Duplicates found for product", body = DuplicateDetectionResultTO),
        (status = 404, description = "Product not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn find_duplicates_by_ean<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(ean): Path<String>,
    Query(params): Query<DuplicateQueryParams>,
) -> Response {
    error_handler(
        (async {
            let config_to: DuplicateDetectionConfigTO = params.into();
            let config = config_to.into();

            // First get the product by EAN
            let auth = crate::extract_auth_context(Some(context))?;
            let product = rest_state
                .product_service()
                .get_by_ean(&ean, auth.clone(), None)
                .await
                .map_err(RestError::from)?;

            // Then find duplicates for that product
            let result = rest_state
                .duplicate_detection_service()
                .find_duplicates(&product, Some(config), auth, None)
                .await
                .map_err(RestError::from)?;

            let response_result = DuplicateDetectionResultTO::from(result);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response_result).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state, request))]
#[utoipa::path(
    post,
    tag = "Duplicate Detection",
    path = "/check",
    params(
        ("similarity_threshold" = Option<f64>, Query, description = "Minimum similarity threshold (0.0-1.0). Default: 0.55. Lower values (0.45) catch more duplicates, higher values (0.7) are more conservative.", example = 0.55),
        ("exact_match_weight" = Option<f64>, Query, description = "Weight for exact matches (0.0-1.0). Reduced because exact matches are rare in real-world data.", example = 0.3),
        ("word_order_weight" = Option<f64>, Query, description = "Weight for word order similarity (0.0-1.0). Important for German text where word order varies.", example = 0.4),
        ("levenshtein_weight" = Option<f64>, Query, description = "Weight for Levenshtein similarity (0.0-1.0). Good for detecting typos like 'süß' vs 'süss'.", example = 0.2),
        ("jaro_winkler_weight" = Option<f64>, Query, description = "Weight for Jaro-Winkler similarity (0.0-1.0). Supplementary algorithm for character transpositions.", example = 0.1),
        ("category_aware" = Option<bool>, Query, description = "Enable category-aware matching. Considers sales_unit and requires_weighing fields.", example = true),
    ),
    request_body = CheckDuplicateRequestTO,
    responses(
        (status = 200, description = "Potential duplicates found", body = Vec<DuplicateMatchTO>),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn check_potential_duplicate<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(params): Query<DuplicateQueryParams>,
    Json(request): Json<CheckDuplicateRequestTO>,
) -> Response {
    error_handler(
        (async {
            let config_to: DuplicateDetectionConfigTO = params.into();
            let config = config_to.into();

            let matches = rest_state
                .duplicate_detection_service()
                .check_potential_duplicate(
                    &request.name,
                    &request.sales_unit,
                    request.requires_weighing,
                    Some(config),
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await
                .map_err(RestError::from)?;

            let response_matches: Vec<DuplicateMatchTO> =
                matches.into_iter().map(DuplicateMatchTO::from).collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response_matches).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(find_all_duplicates, find_duplicates_by_ean, check_potential_duplicate),
    components(schemas(
        DuplicateDetectionResultTO,
        DuplicateMatchTO,
        DuplicateDetectionConfigTO,
        CheckDuplicateRequestTO,
        inventurly_rest_types::AlgorithmScoresTO,
        inventurly_rest_types::MatchConfidenceTO,
        inventurly_rest_types::ProductTO,
    )),
    tags(
        (name = "Duplicate Detection", description = "Product duplicate detection endpoints")
    )
)]
pub struct DuplicateDetectionApiDoc;
