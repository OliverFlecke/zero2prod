mod get;
pub use get::publish_newsletter_html;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, require_login::AuthorizedUser,
    service::flash_message::FlashMessage,
};
use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use http::StatusCode;
use sqlx::PgPool;
use std::sync::Arc;

/// Publish a newsletter with the given title and content.
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(db_pool, email_client, flash, body),
    fields(user_id=tracing::field::Empty),
)]
pub async fn publish_newsletter(
    user: AuthorizedUser,
    State(db_pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    flash: FlashMessage,
    Form(body): Form<BodyData>,
) -> Result<impl IntoResponse, PublishNewsletterError> {
    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .map_err(PublishNewsletterError::FailedToGetConfirmedSubscribers)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &body.title, &body.content, &body.content)
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

    Ok((
        flash.set_message("The newsletter issue has been published".to_string()),
        Redirect::to("/admin/newsletters"),
    )
        .into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: String,
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
}

impl IntoResponse for PublishNewsletterError {
    fn into_response(self) -> Response {
        tracing::error!("{self:?}");
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
