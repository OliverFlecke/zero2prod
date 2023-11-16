use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use chrono::{DateTime, NaiveDateTime};
use lazy_static::lazy_static;
use sqlx::PgPool;
use std::sync::Arc;

lazy_static! {
    static ref VERSION: String = env!("CARGO_PKG_VERSION").to_string();
    static ref BUILD_GIT_SHA: String = env!("VERGEN_GIT_SHA").to_string();
    static ref BUILD_TIMESTAMP: NaiveDateTime =
        DateTime::parse_from_rfc3339(env!("VERGEN_BUILD_TIMESTAMP"))
            .expect("Failed to parse build timestamp")
            .naive_utc();
}

/// Create a router to serve health checks.
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(is_alive))
        .route("/info", get(build_info))
        .route("/status", get(status))
}

/// Simple `is_alive` endpoint that will always return a 200 OK.
/// Used to indicate when the webserver is up and running.
#[tracing::instrument]
#[utoipa::path(
    get,
    path = "/health",
    responses((status = OK, description = "Check if service is alive"))
)]
async fn is_alive() -> StatusCode {
    tracing::debug!("Service is alive");
    StatusCode::OK
}

/// Status endpoint to whether all required depedencies are working.
#[tracing::instrument(skip(db_pool))]
#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = OK, description = "Current status of all dependent services", body = Status)
    )
)]
async fn status(
    State(db_pool): State<Arc<PgPool>>,
    State(redis_client): State<Arc<redis::Client>>,
) -> Json<Status> {
    // TODO: Can this be done once instead of everytime to report the
    // connection status? On the other hand, it should also report a up-to-date
    // response.
    let is_db_connected = db_pool
        .acquire()
        .await
        .map_err(|e| {
            tracing::error!("{:?}", e);
            e
        })
        .is_ok();
    let is_redis_connected = redis_client
        .get_async_std_connection()
        .await
        .map_err(|e| {
            tracing::error!("{:?}", e);
            e
        })
        .is_ok();

    let status = Status {
        is_db_connected,
        is_redis_connected,
    };
    tracing::info!("Status: {:?}", status);
    Json(status)
}

/// Endpoint to get current information about the server's version.
#[tracing::instrument]
#[utoipa::path(
    get,
    path = "/build_info",
    responses(
        (status = OK, description = "Build info for this service", body = BuildInfo)
    )
)]
async fn build_info<'a>() -> Json<BuildInfo<'a>> {
    Json(BuildInfo {
        version: VERSION.as_str(),
        build_timestamp: &BUILD_TIMESTAMP,
        build: BUILD_GIT_SHA.as_str(),
    })
}

/// Overall status of required dependencies.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Status {
    /// `true` when the service is successfully connected to its db.
    is_db_connected: bool,
    /// `true` when the service is successfully connected to redis.
    is_redis_connected: bool,
}

/// Contains all relevant information about the current deployment.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BuildInfo<'a> {
    /// Version of the service.
    version: &'a str,
    /// Datetime in UTC when the service was build.
    build_timestamp: &'a NaiveDateTime,
    /// SHA hash for the build.
    build: &'a str,
}
