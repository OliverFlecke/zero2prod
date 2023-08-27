use axum::{http::StatusCode, routing::get, Router};

/// Create a router to serve health checks.
pub fn create_router() -> Router {
    Router::new().route("/", get(is_alive))
}

/// Simple "is_alive" endpoint that will always return a 200 OK.
/// Used to indicate when the webserver is up and running.
async fn is_alive() -> StatusCode {
    StatusCode::OK
}
