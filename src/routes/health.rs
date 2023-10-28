use axum::{http::StatusCode, routing::get, Json, Router};
use lazy_static::lazy_static;

lazy_static! {
    static ref VERSION: String = env!("CARGO_PKG_VERSION").to_string();
}

/// Create a router to serve health checks.
pub fn create_router() -> Router {
    Router::new()
        .route("/", get(is_alive))
        .route("/status", get(status))
}

/// Simple "is_alive" endpoint that will always return a 200 OK.
/// Used to indicate when the webserver is up and running.
async fn is_alive() -> StatusCode {
    tracing::debug!("Service is alive");
    StatusCode::OK
}

#[derive(serde::Serialize)]
struct AppStatus<'a> {
    version: &'a str,
}

/// Endpoint to get current information about the server's version.
async fn status<'a>() -> Json<AppStatus<'a>> {
    Json(AppStatus {
        version: VERSION.as_str(),
    })
}
