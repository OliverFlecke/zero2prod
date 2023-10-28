use axum::{http::StatusCode, routing::get, Json, Router};
use chrono::{DateTime, NaiveDateTime};
use lazy_static::lazy_static;

lazy_static! {
    static ref VERSION: String = env!("CARGO_PKG_VERSION").to_string();
    static ref BUILD_GIT_SHA: String = env!("VERGEN_GIT_SHA").to_string();
    static ref BUILD_TIMESTAMP: NaiveDateTime =
        DateTime::parse_from_rfc3339(env!("VERGEN_BUILD_TIMESTAMP"))
            .expect("Failed to parse build timestamp")
            .naive_utc();
}

/// Create a router to serve health checks.
pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(is_alive))
        .route("/info", get(build_info))
}

/// Simple "is_alive" endpoint that will always return a 200 OK.
/// Used to indicate when the webserver is up and running.
async fn is_alive() -> StatusCode {
    tracing::debug!("Service is alive");
    StatusCode::OK
}

#[derive(serde::Serialize)]
struct BuildInfo<'a> {
    version: &'a str,
    build_timestamp: &'a NaiveDateTime,
    build: &'a str,
}

/// Endpoint to get current information about the server's version.
async fn build_info<'a>() -> Json<BuildInfo<'a>> {
    Json(BuildInfo {
        version: VERSION.as_str(),
        build_timestamp: &BUILD_TIMESTAMP,
        build: BUILD_GIT_SHA.as_str(),
    })
}
