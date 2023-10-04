use crate::{
    authorization::{Credentials, CredentialsError},
    service::flash_message::FlashMessage,
    state::session::Session,
};
use axum::{
    body::Empty,
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use http::{header, StatusCode};
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;

#[tracing::instrument(
    name = "Perform a login attempt",
    skip(form, pool, flash_message, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
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

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/admin/dashboard")
        .body(Empty::default())
        .unwrap()
        .into_response()
}

fn login_redirect(flash_message: FlashMessage, e: LoginError) -> Response {
    (
        flash_message.set_message(e.to_string()),
        Redirect::to("/login"),
    )
        .into_response()
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
