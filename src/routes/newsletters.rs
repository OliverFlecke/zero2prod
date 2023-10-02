use crate::{
    authorization::{build_auth_error, Credentials, CredentialsError},
    domain::SubscriberEmail,
    email_client::EmailClient,
    state::AppState,
};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use http::StatusCode;
use sqlx::PgPool;
use std::sync::Arc;

/// Create a router to serve endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/", post(publish_newsletter))
}

/// Publish a newsletter with the given title and content.
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(db_pool, email_client, body),
    fields(user_id=tracing::field::Empty),
)]
async fn publish_newsletter(
    credentials: Credentials,
    State(db_pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishNewsletterError> {
    let user_id = credentials
        .validate_credentials(&db_pool)
        .await
        .map_err(PublishNewsletterError::AuthError)?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .map_err(PublishNewsletterError::FailedToGetConfirmedSubscribers)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .map_err(|e| PublishNewsletterError::FailedToSendEmail(e, subscriber.email))?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber.\
                    Their stored contact details are invalid"
                );
            }
        }
    }

    Ok(StatusCode::OK)
}

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

/// Get all confirmed subscribers from the database.
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let rows = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
        .fetch_all(db_pool)
        .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

/// Represent the different possible errors that can happen during publishing
/// a newsletter.
#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error("Failed to get confirmed subscribers")]
    FailedToGetConfirmedSubscribers(#[source] sqlx::Error),
    #[error("Failed to send newsletter issue to {1}")]
    FailedToSendEmail(#[source] reqwest::Error, SubscriberEmail),
    #[error("Failed to validate credentials")]
    AuthError(#[source] CredentialsError),
}

impl IntoResponse for PublishNewsletterError {
    fn into_response(self) -> Response {
        match self {
            Self::FailedToGetConfirmedSubscribers(_) | Self::FailedToSendEmail(_, _) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
            Self::AuthError(_) => build_auth_error(self.to_string()),
        }
    }
}
