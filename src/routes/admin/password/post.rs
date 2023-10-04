use crate::{
    authorization::{Credentials, CredentialsError},
    require_login::AuthorizedUser,
    service::{flash_message::FlashMessage, user::UserService},
};
use anyhow::Context;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use http::StatusCode;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use std::sync::Arc;

#[tracing::instrument(name = "Change password", skip(flash, data, user_service))]
pub async fn change_password(
    State(pool): State<Arc<PgPool>>,
    State(user_service): State<UserService>,
    flash: FlashMessage,
    user: AuthorizedUser,
    Form(data): Form<FormData>,
) -> Result<Response, ChangePasswordError> {
    if data.new_password.expose_secret() != data.new_password_check.expose_secret() {
        let flash = flash.set_message(
            "You entered two different new passwords - the field values must match.".to_string(),
        );
        return Ok((flash, Redirect::to("/admin/password")).into_response());
    }

    let username = user_service
        .get_username(user.user_id())
        .await
        .context("Failed to retreive username")
        .map_err(ChangePasswordError::Unexpected)?;

    let credentials = Credentials::new(username, data.current_password);
    if let Err(e) = credentials.validate_credentials(&pool).await {
        return match e {
            CredentialsError::InvalidPassword(_) => {
                let flash = flash.set_message("The current password is incorrect.".to_string());
                Ok((flash, Redirect::to("/admin/password")).into_response())
            }
            _ => Err(ChangePasswordError::Unexpected(anyhow::anyhow!(e))),
        };
    }

    todo!()
}

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum ChangePasswordError {
    #[error("Unexpected error")]
    Unexpected(#[source] anyhow::Error),
}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> askama_axum::Response {
        tracing::error!("{self:?}");
        match self {
            ChangePasswordError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
