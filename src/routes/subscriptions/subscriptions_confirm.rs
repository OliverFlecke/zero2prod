use axum::extract::Query;
use http::StatusCode;

#[derive(Debug, serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber")]
pub async fn confirm(Query(parameters): Query<Parameters>) -> StatusCode {
    StatusCode::OK
}
