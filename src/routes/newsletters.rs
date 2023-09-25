use crate::{domain::SubscriberEmail, email_client::EmailClient, state::AppState};
use axum::{
    async_trait,
    body::Full,
    extract::{FromRequestParts, State},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::Engine;
use http::{
    header::{self, ToStrError},
    StatusCode,
};
use secrecy::Secret;
use sqlx::PgPool;
use std::{string::FromUtf8Error, sync::Arc};

/// Create a router to serve endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/", post(publish_newsletter))
}

/// Publish a newsletter with the given title and content.
#[tracing::instrument(name = "Publish newsletter", skip(db_pool))]
async fn publish_newsletter(
    credentials: Credentials,
    State(db_pool): State<Arc<PgPool>>,
    State(email_client): State<Arc<EmailClient>>,
    Json(body): Json<BodyData>,
) -> Result<impl IntoResponse, PublishNewsletterError> {
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
}

impl IntoResponse for PublishNewsletterError {
    fn into_response(self) -> Response {
        let status_code = match self {
            PublishNewsletterError::FailedToGetConfirmedSubscribers(_)
            | PublishNewsletterError::FailedToSendEmail(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, self.to_string()).into_response()
    }
}

#[derive(Debug)]
struct Credentials {
    username: String,
    password: Secret<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Credentials
where
    S: Send + Sync,
{
    type Rejection = BasicAuthError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or(BasicAuthError::MissingHeader)?
            .to_str()
            .map_err(BasicAuthError::NotValidUTF8String)?;

        let base64encoded_segment = header_value
            .strip_prefix("Basic ")
            .ok_or(BasicAuthError::SchemeNotBasic)?;
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(base64encoded_segment)
            .map_err(|_| BasicAuthError::FailedToBase64Decode)?;
        let decoded_credentials = String::from_utf8(decoded_bytes)
            .map_err(BasicAuthError::DecodedCredentialStringNotUTF8)?;

        let mut credentials = decoded_credentials.splitn(2, ':');
        let username = credentials
            .next()
            .ok_or(BasicAuthError::MissingUsername)?
            .to_string();
        let password = credentials
            .next()
            .ok_or(BasicAuthError::MissingPassword)?
            .to_string();

        Ok(Credentials {
            username,
            password: Secret::new(password),
        })
    }
}

#[derive(thiserror::Error)]
pub enum BasicAuthError {
    #[error("The 'Authorization' header was missing")]
    MissingHeader,
    #[error("The 'Authorization' header was not a valid UTF8 string")]
    NotValidUTF8String(#[source] ToStrError),
    #[error("The authorization scheme was not 'Basic'")]
    SchemeNotBasic,
    #[error("Failed to base64-decode 'Basic' credentials")]
    FailedToBase64Decode,
    #[error("The decoded credential string is not valid UTF8")]
    DecodedCredentialStringNotUTF8(#[source] FromUtf8Error),
    #[error("A username must be provided in 'Basic' Auth")]
    MissingUsername,
    #[error("A password must be provided in 'Basic' Auth")]
    MissingPassword,
}

impl IntoResponse for BasicAuthError {
    fn into_response(self) -> Response {
        Response::builder()
            .header(header::WWW_AUTHENTICATE, r#"Basic realm="publish""#)
            .status(StatusCode::UNAUTHORIZED)
            .body(Full::from(self.to_string()))
            .unwrap()
            .into_response()
    }
}
