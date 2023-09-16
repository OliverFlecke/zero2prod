use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    state::AppState,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Form, Router};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;

        Ok(Self { email, name })
    }
}

/// Create a router to serve endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/", post(subscribe))
}

/// Subscribe to the newsletter with an email and name.
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
async fn subscribe(
    State(pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = match form.try_into() {
        Ok(x) => x,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY,
    };

    if insert_subscriber(pool.as_ref(), &new_subscriber)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_email_confirmation(email_client, new_subscriber)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

/// Send an email to the new subscriber with a link for them to confirm the
/// subscription.
#[tracing::instrument(
    name = "Send a email confirmation to a new subscriber",
    skip(email_client, new_subscriber)
)]
async fn send_email_confirmation(
    email_client: Arc<EmailClient>,
    new_subscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirmation_link = "https://there-is-no-such-domain.com/subscriptions/confirm";
    let html_body = format!(
        "Welcome to our newsletter!<br/> \
                Click <a href=\"{confirmation_link}\">here</a> to confirm."
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &text_body)
        .await?;

    Ok(())
}

/// Insert a new subscriber into the database.
#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(new_subscriber, pool)
)]
async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status)
           VALUES($1, $2, $3, $4, 'pending_confirmation')"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    tracing::info!("New subscriber details have been saved");
    Ok(())
}
