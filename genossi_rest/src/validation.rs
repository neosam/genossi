use axum::{
    body::Body,
    extract::State,
    response::Response,
    routing::get,
    Extension, Router,
};
use genossi_rest_types::ValidationResultTO;
use genossi_service::validation::ValidationService;
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/", get(get_validation::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(get_validation),
    components(schemas(
        ValidationResultTO,
        genossi_rest_types::UnmatchedTransferTO,
        genossi_rest_types::SharesMismatchTO,
        genossi_rest_types::MissingEntryActionTO,
        genossi_rest_types::ExitDateMismatchTO,
        genossi_rest_types::ActiveMemberNoSharesTO,
        genossi_rest_types::DuplicateMemberNumberTO,
        genossi_rest_types::ExitedMemberWithSharesTO,
        genossi_rest_types::MigratedFlagMismatchTO,
    )),
    tags((name = "Validation", description = "Data integrity validation"))
)]
pub struct ApiDoc;

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Validation",
    path = "",
    responses(
        (status = 200, description = "Validation results", body = ValidationResultTO),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_validation<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let result = rest_state
                .validation_service()
                .validate(crate::extract_auth_context(Some(context))?)
                .await?;
            let to = ValidationResultTO::from(&result);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}
