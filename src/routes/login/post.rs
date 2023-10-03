use crate::{
    authorization::{Credentials, CredentialsError},
    state::session::Session,
};
use axum::{
    body::Empty,
    extract::State,
    response::{IntoResponse, Response},
    Form,
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use http::{header, StatusCode};
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

#[tracing::instrument(
    name = "Perform a login attempt",
    skip(form, pool, cookie_jar),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    cookie_jar: SignedCookieJar,
    mut session: Session,
    Form(form): Form<FormData>,
) -> Response {
    let credentials: Credentials = form.into();
    tracing::Span::current().record("username", &tracing::field::display(credentials.username()));

    let user_id = match credentials
        .validate_credentials(&pool)
        .await
        .map_err(|e| match e {
            CredentialsError::UnknownUsername(_) | CredentialsError::InvalidPassword(_) => {
                LoginError::AuthError(e)
            }
            _ => LoginError::Unexpected(anyhow::anyhow!(e)),
        }) {
        Ok(user_id) => user_id,
        Err(e) => return login_redirect(cookie_jar, e),
    };

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    session.regenerate();
    if let Err(e) = session
        .insert_user_id(user_id)
        .map_err(|e| LoginError::Unexpected(anyhow::anyhow!(e)))
    {
        return login_redirect(cookie_jar, e);
    }

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/admin/dashboard")
        .body(Empty::default())
        .unwrap()
        .into_response()
}

fn login_redirect(cookie_jar: SignedCookieJar, e: LoginError) -> Response {
    let cookie = Cookie::build("_flash", e.to_string())
        // Set the cookie to expire straight away so only the first
        // GET request to `/login` will contain the error message.
        .max_age(cookie::time::Duration::seconds(1))
        .secure(true)
        .http_only(true)
        .finish();

    let response = Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login")
        .body(Empty::default())
        .unwrap()
        .into_response();

    (cookie_jar.add(cookie), response).into_response()
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
    #[error("Unexpected error")]
    Unexpected(#[source] anyhow::Error),
}
