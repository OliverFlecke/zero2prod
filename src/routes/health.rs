use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use chrono::{DateTime, NaiveDateTime};
use lazy_static::lazy_static;
use sqlx::PgPool;
use std::sync::Arc;
use utoipa::ToSchema;

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

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct Status {
    db_connected: bool,
    // TODO: Add field to report redis status
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
async fn status(State(db_pool): State<Arc<PgPool>>) -> Json<Status> {
    // TODO: Can this be done once instead of everytime to report the
    // connection status? On the other hand, it should also report a up-to-date
    // response.
    let db_connected = db_pool
        .acquire()
        .await
        .map_err(|e| {
            tracing::error!("{:?}", e);
            e
        })
        .is_ok();

    let status = Status { db_connected };
    tracing::info!("Status: {:?}", status);
    Json(status)
}

#[derive(serde::Serialize, ToSchema)]
pub struct BuildInfo<'a> {
    version: &'a str,
    build_timestamp: &'a NaiveDateTime,
    build: &'a str,
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
