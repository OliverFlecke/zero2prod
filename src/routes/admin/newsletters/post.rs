use crate::{
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    require_login::AuthorizedUser,
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
    let idempotency_key: IdempotencyKey = body
        .idempotency_key
        .clone()
        .try_into()
        .map_err(PublishNewsletterError::InvalidIdempotencyKey)?;

    // Return early if we have a saved response in the database for the same request.
    let transaction = match try_processing(&db_pool, &idempotency_key, user.user_id())
        .await
        .map_err(PublishNewsletterError::UnableToGetSavedResponse)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok((success_message(flash), saved_response).into_response());
        }
    };

    send_email_to_subscribers(&email_client, &db_pool, &body).await?;

    let response = (success_message(flash), Redirect::to("/admin/newsletters")).into_response();

    let response = save_response(transaction, &idempotency_key, user.user_id(), response)
        .await
        .map_err(PublishNewsletterError::FailedToSaveResponseWithIdempotencyKey)?;

    Ok(response)
}

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

fn success_message(flash: FlashMessage) -> FlashMessage {
    flash.set_message("The newsletter issue has been published".to_string())
}

/// Send out emails to all the subscribres.
#[tracing::instrument(name = "Send email to subscribers", skip(email_client, db_pool, body))]
async fn send_email_to_subscribers(
    email_client: &EmailClient,
    db_pool: &PgPool,
    body: &BodyData,
) -> Result<(), PublishNewsletterError> {
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

    Ok(())
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
    #[error("Invalid idempotency key")]
    InvalidIdempotencyKey(#[source] anyhow::Error),
    #[error("Unable to get saved response")]
    UnableToGetSavedResponse(#[source] anyhow::Error),
    #[error("Failed to save response with idempotency key")]
    FailedToSaveResponseWithIdempotencyKey(#[source] anyhow::Error),
}

impl IntoResponse for PublishNewsletterError {
    fn into_response(self) -> Response {
        tracing::error!("{self:?}");

        match self {
            Self::FailedToGetConfirmedSubscribers(_)
            | Self::FailedToSendEmail(_, _)
            | Self::UnableToGetSavedResponse(_)
            | Self::FailedToSaveResponseWithIdempotencyKey(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            Self::InvalidIdempotencyKey(_) => StatusCode::BAD_REQUEST.into_response(),
        }
    }
}
