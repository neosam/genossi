use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::routing::get;
use axum::{extract::State, response::Response};
use axum::{Extension, Router};
use inventurly_rest_types::{InventurProductReportItemTO, InventurStatisticsTO, RackMeasuredTO};
use inventurly_service::inventur_report::InventurReportService;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_product_report,
        get_product_report_csv,
        get_statistics
    ),
    components(
        schemas(InventurProductReportItemTO, InventurStatisticsTO, RackMeasuredTO)
    ),
    tags(
        (name = "Inventur Report", description = "Cumulative inventory report endpoints")
    )
)]
pub struct ApiDoc;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/{inventur_id}/report", get(get_product_report::<RestState>))
        .route(
            "/{inventur_id}/report/csv",
            get(get_product_report_csv::<RestState>),
        )
        .route(
            "/{inventur_id}/statistics",
            get(get_statistics::<RestState>),
        )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Inventur Report",
    path = "/{inventur_id}/report",
    params(
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Cumulative product report", body = [InventurProductReportItemTO]),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inventur not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_product_report<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(inventur_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let items: Arc<[InventurProductReportItemTO]> = rest_state
                .inventur_report_service()
                .get_product_report(inventur_id, crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(InventurProductReportItemTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&items).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Inventur Report",
    path = "/{inventur_id}/report/csv",
    params(
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "CSV export of cumulative product report", content_type = "text/csv"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inventur not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_product_report_csv<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(inventur_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let csv_content = rest_state
                .inventur_report_service()
                .get_product_report_csv(
                    inventur_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/csv; charset=utf-8")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"inventur-report-{}.csv\"", inventur_id),
                )
                .body(Body::new(csv_content))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Inventur Report",
    path = "/{inventur_id}/statistics",
    params(
        ("inventur_id" = Uuid, Path, description = "Inventur ID"),
    ),
    responses(
        (status = 200, description = "Inventur statistics", body = InventurStatisticsTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inventur not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_statistics<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(inventur_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let stats = rest_state
                .inventur_report_service()
                .get_statistics(inventur_id, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let stats_to = InventurStatisticsTO::from(&stats);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&stats_to).unwrap()))
                .unwrap())
        })
        .await,
    )
}
