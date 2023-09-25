use crate::state::ApplicationBaseUrl;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use http::StatusCode;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Endpoint for user to hit when confirming their subscription to the newsletter.
#[tracing::instrument(name = "Confirm a pending subscriber", skip(db_pool))]
pub async fn confirm(
    State(host): State<Arc<ApplicationBaseUrl>>,
    State(db_pool): State<Arc<PgPool>>,
    Query(parameters): Query<Parameters>,
) -> Result<StatusCode, ConfirmError> {
    let Some(subscriber_id) =
        get_subscriber_id_from_token(&db_pool, &parameters.subscription_token).await?
    else {
        return Err(ConfirmError::SubscriberNotFoundForToken(
            parameters.subscription_token,
        ));
    };

    tracing::info!("Subscriber found: {subscriber_id}");
    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .map_err(ConfirmError::FailedToConfirmSubscriber)?;
    Ok(StatusCode::OK)
}

/// Update the status of the given `subscriber_id` to be confirmed.
#[tracing::instrument(name = "Make subscriber as confirmed", skip(pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;

    tracing::info!("Subscriber confirmed");

    Ok(())
}

/// Retreive the subscriber id from the database that matches the given
/// `subscription_token`.
#[tracing::instrument(name = "Get subscriber_id from token", skip(pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, ConfirmError> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(ConfirmError::FailedToGetToken)?;

    Ok(result.map(|x| x.subscriber_id))
}

/// Errors that can occure during confirmation of a subscriber.
#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("Failed to retreive token")]
    FailedToGetToken(#[source] sqlx::Error),
    #[error("Failed to confirm subscriber")]
    FailedToConfirmSubscriber(#[source] sqlx::Error),
    #[error("Subscriber not found for token: {0}")]
    SubscriberNotFoundForToken(String),
}

impl IntoResponse for ConfirmError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{self:?}");

        let status_code = match self {
            ConfirmError::SubscriberNotFoundForToken(_) => StatusCode::UNAUTHORIZED,
            ConfirmError::FailedToConfirmSubscriber(_) | ConfirmError::FailedToGetToken(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        (status_code, self.to_string()).into_response()
    }
}
