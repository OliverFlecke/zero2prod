use self::auth::{build_auth_error, Credentials, CredentialsError};
use crate::{domain::SubscriberEmail, email_client::EmailClient, state::AppState};
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
    name = "Publish newsletter",
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

// Authentication
pub mod auth {
    use anyhow::Context;
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    use axum::{
        async_trait,
        body::Full,
        extract::FromRequestParts,
        response::{IntoResponse, Response},
    };
    use base64::Engine;
    use http::{
        header::{self, ToStrError},
        StatusCode,
    };
    use secrecy::{ExposeSecret, Secret};
    use sqlx::PgPool;
    use std::string::FromUtf8Error;

    use crate::telemetry::spawn_blocking_with_tracing;

    #[derive(Debug)]
    pub struct Credentials {
        username: String,
        password: Secret<String>,
    }

    impl Credentials {
        #[tracing::instrument(name = "Validate credentials", skip(self, pool))]
        pub async fn validate_credentials(
            self,
            pool: &PgPool,
        ) -> Result<uuid::Uuid, CredentialsError> {
            let mut user_id = None;
            let mut expected_password_hash = Secret::new(
                "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                    .to_string(),
            );

            if let Some((stored_user_id, stored_password_hash)) =
                get_stored_credentials(&self.username, pool).await?
            {
                user_id = Some(stored_user_id);
                expected_password_hash = stored_password_hash;
            }

            spawn_blocking_with_tracing(move || {
                verify_password_hash(expected_password_hash, self.password)
            })
            .await
            .context("Failed to spawn blocking task")
            .map_err(CredentialsError::UnexpectedError)??;

            user_id.ok_or_else(|| CredentialsError::UnknownUsername(self.username))
        }
    }

    #[tracing::instrument(
        name = "Verify password hash",
        skip(expected_password_hash, password_candidate)
    )]
    fn verify_password_hash(
        expected_password_hash: Secret<String>,
        password_candidate: Secret<String>,
    ) -> Result<(), CredentialsError> {
        let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
            .map_err(CredentialsError::FailedToGetExpectedHash)?;

        Argon2::default()
            .verify_password(
                password_candidate.expose_secret().as_bytes(),
                &expected_password_hash,
            )
            .map_err(CredentialsError::InvalidPassword)?;

        Ok(())
    }

    /// Get the stored user id and its corresponding password hash from the
    /// database.
    #[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
    async fn get_stored_credentials(
        username: &str,
        pool: &PgPool,
    ) -> Result<Option<(uuid::Uuid, Secret<String>)>, CredentialsError> {
        Ok(sqlx::query!(
            r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
            username,
        )
        .fetch_optional(pool)
        .await
        .map_err(CredentialsError::DbError)?
        .map(|row| (row.user_id, Secret::new(row.password_hash))))
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

    pub fn build_auth_error(body: String) -> Response {
        Response::builder()
            .header(header::WWW_AUTHENTICATE, r#"Basic realm="publish""#)
            .status(StatusCode::UNAUTHORIZED)
            .body(Full::from(body))
            .unwrap()
            .into_response()
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
            build_auth_error(self.to_string())
        }
    }

    #[derive(thiserror::Error)]
    pub enum CredentialsError {
        #[error("Unexpected database error")]
        DbError(#[source] sqlx::Error),
        #[error("Unknown username: '{0}'")]
        UnknownUsername(String),
        #[error("Invalid password")]
        InvalidPassword(#[source] argon2::password_hash::Error),
        #[error("Failed to create expected hash")]
        FailedToGetExpectedHash(#[source] argon2::password_hash::Error),
        #[error("Unexpected error")]
        UnexpectedError(#[source] anyhow::Error),
    }
}
