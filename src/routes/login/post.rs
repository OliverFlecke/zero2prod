use crate::{
    authorization::{Credentials, CredentialsError},
    service::flash_message::FlashMessage,
    state::session::Session,
};
use axum::{
    body::Body,
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use http::{header, StatusCode};
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

/// POST a login attempt with a pair of user credentials.
#[tracing::instrument(
    name = "Perform a login attempt",
    skip(form, pool, flash_message, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
#[utoipa::path(
    post,
    path = "/login",
    params(FormData),
    responses(
        (
            status = SEE_OTHER,
            description = "On a successfull login, redirects to `/admin/dashboard`. On a incorrect login attempt, redirects back to `/login` with an error message",
        ),
    )
)]
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    flash_message: FlashMessage,
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
        Err(e) => return login_redirect(flash_message, e),
    };

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    session.regenerate();
    if let Err(e) = session
        .insert_user_id(user_id)
        .map_err(|e| LoginError::Unexpected(anyhow::anyhow!(e)))
    {
        return login_redirect(flash_message, e);
    }

    tracing::info!("User successfully logged in");
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/admin/dashboard")
        .body(Body::empty())
        .unwrap()
        .into_response()
}

/// Redirects back to the login screen with an error message extracted from
/// the `LoginError`. Should be used when the login attempt failed.
fn login_redirect(flash_message: FlashMessage, e: LoginError) -> Response {
    tracing::error!("{:?}", e);

    (
        flash_message.set_message(e.to_string()),
        Redirect::to("/login"),
    )
        .into_response()
}

/// Parameters with credentials for a user to login.
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

impl From<FormData> for Credentials {
    fn from(value: FormData) -> Self {
        Self::new(value.username, value.password)
    }
}

/// Errors that can occure during a login.
#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] CredentialsError),
    #[error("Unexpected error")]
    Unexpected(#[source] anyhow::Error),
}
