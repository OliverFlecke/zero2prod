use crate::state::ApplicationBaseUrl;
use axum::extract::{Query, State};
use http::StatusCode;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Endpoint for user to hit when confirming their subscription to the newsletter.
#[tracing::instrument(name = "Confirm a pending subscriber")]
pub async fn confirm(
    State(host): State<Arc<ApplicationBaseUrl>>,
    State(db_pool): State<Arc<PgPool>>,
    Query(parameters): Query<Parameters>,
) -> StatusCode {
    let id = match get_subscriber_id_from_token(&db_pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        None => StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => match confirm_subscriber(&db_pool, subscriber_id).await {
            Ok(_) => StatusCode::OK,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        },
    }
}

/// Update the status of the given `subscriber_id` to be confirmed.
#[tracing::instrument(name = "Make subscriber as confirmed", skip(pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}

/// Retreive the subscriber id from the database that matches the given
/// `subscription_token`.
#[tracing::instrument(name = "Get subscriber_id from token", skip(pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    Ok(result.map(|x| x.subscriber_id))
}
