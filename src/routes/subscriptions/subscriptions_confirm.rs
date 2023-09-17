use crate::state::ApplicationBaseUrl;
use axum::extract::{Query, State};
use http::StatusCode;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Endpoint for user to hit when confirming their subscription to the newsletter.
#[tracing::instrument(name = "Confirm a pending subscriber")]
pub async fn confirm(
    State(host): State<Arc<ApplicationBaseUrl>>,
    Query(parameters): Query<Parameters>,
) -> StatusCode {
    StatusCode::OK
}
