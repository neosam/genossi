use axum::{extract::State, response::Response, Extension, Router, routing::get};

#[derive(Clone)]
struct TestState;

#[derive(Clone, Debug)]
struct TestContext;

async fn test_handler(
    State(state): State<TestState>,
    Extension(context): Extension<TestContext>,
) -> Response {
    Response::new("Hello".into())
}

fn main() {
    let router = Router::new()
        .route("/", get(test_handler))
        .with_state(TestState);
    println!("Test compiled successfully");
}