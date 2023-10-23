use crate::{
    domain::SubscriberEmail,
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
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: String,
    idempotency_key: String,
}

/// Publish a newsletter with the given title and content.
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(db_pool, flash, body),
    fields(user_id=tracing::field::Empty),
)]
pub async fn publish_newsletter(
    user: AuthorizedUser,
    State(db_pool): State<Arc<PgPool>>,
    flash: FlashMessage,
    Form(body): Form<BodyData>,
) -> Result<impl IntoResponse, PublishNewsletterError> {
    let idempotency_key: IdempotencyKey = body
        .idempotency_key
        .clone()
        .try_into()
        .map_err(PublishNewsletterError::InvalidIdempotencyKey)?;

    // Return early if we have a saved response in the database for the same request.
    let mut transaction = match try_processing(&db_pool, &idempotency_key, user.user_id())
        .await
        .map_err(PublishNewsletterError::UnableToGetSavedResponse)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok((success_message(flash), saved_response).into_response());
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &body.title, &body.content)
        .await
        .map_err(PublishNewsletterError::FailedToInsertNewsletterIssue)?;

    enqueue_delivery_tasks(&mut transaction, &issue_id)
        .await
        .map_err(PublishNewsletterError::FailedToEnqueueDeliveryTasks)?;

    let response = (success_message(flash), Redirect::to("/admin/newsletters")).into_response();

    let response = save_response(transaction, &idempotency_key, user.user_id(), response)
        .await
        .map_err(PublishNewsletterError::FailedToSaveResponseWithIdempotencyKey)?;

    Ok(response)
}

/// Insert a newsletter issue to be sent out to all subscribers.
#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            published_at
        )
        VALUES ($1, $2, $3, now())"#,
        newsletter_issue_id,
        title,
        text_content,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(newsletter_issue_id)
}

/// Enqueue delivery tasks for newsletter issues
#[tracing::instrument(skip(transaction))]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

fn success_message(flash: FlashMessage) -> FlashMessage {
    flash.set_message("The newsletter issue has been published".to_string())
}

/// Represent the different possible errors that can happen during publishing
/// a newsletter.
#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error("Invalid idempotency key")]
    InvalidIdempotencyKey(#[source] anyhow::Error),
    #[error("Unable to get saved response")]
    UnableToGetSavedResponse(#[source] anyhow::Error),
    #[error("Failed to save response with idempotency key")]
    FailedToSaveResponseWithIdempotencyKey(#[source] anyhow::Error),
    #[error("Failed to insert newsletter issue")]
    FailedToInsertNewsletterIssue(#[source] sqlx::Error),
    #[error("Failed to enqueue deliver tasks for newsletter issue delivery")]
    FailedToEnqueueDeliveryTasks(#[source] sqlx::Error),
}

impl IntoResponse for PublishNewsletterError {
    fn into_response(self) -> Response {
        tracing::error!("{self:?}");

        match self {
            Self::UnableToGetSavedResponse(_)
            | Self::FailedToSaveResponseWithIdempotencyKey(_)
            | Self::FailedToInsertNewsletterIssue(_)
            | Self::FailedToEnqueueDeliveryTasks(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            Self::InvalidIdempotencyKey(_) => StatusCode::BAD_REQUEST.into_response(),
        }
    }
}
