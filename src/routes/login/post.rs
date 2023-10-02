use crate::{
    authorization::{Credentials, CredentialsError},
    state::HmacSecret,
};
use axum::{
    body::Empty,
    extract::State,
    response::{IntoResponse, Response},
    Form,
};
use hmac::{Hmac, Mac};
use http::{header, StatusCode};
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

#[tracing::instrument(
    skip(form, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    State(hmac_secret): State<Arc<HmacSecret>>,
    Form(form): Form<FormData>,
) -> Response {
    let credentials: Credentials = form.into();
    tracing::Span::current().record("username", &tracing::field::display(credentials.username()));

    match credentials.validate_credentials(&pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(header::LOCATION, "/")
                .body(Empty::default())
                .unwrap()
                .into_response()
        }
        Err(e) => {
            build_error_response_with_redirect(LoginError::AuthError(e), hmac_secret.as_ref())
                .into_response()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

impl From<FormData> for Credentials {
    fn from(value: FormData) -> Self {
        Self::new(value.username, value.password)
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] CredentialsError),
}

fn build_error_response_with_redirect(error: LoginError, hmac_secret: &HmacSecret) -> Response {
    let query_string = format!("error={}", urlencoding::Encoded::new(error.to_string()));

    let hmac_tag = {
        let mut mac =
            Hmac::<sha3::Sha3_256>::new_from_slice(hmac_secret.0.expose_secret().as_bytes())
                .unwrap();
        mac.update(query_string.as_bytes());
        mac.finalize().into_bytes()
    };

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(
            header::LOCATION,
            format!("/login?{}&tag={hmac_tag:x}", query_string),
        )
        .body(Empty::default())
        .unwrap()
        .into_response()
}
