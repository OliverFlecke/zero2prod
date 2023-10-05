use crate::{
    authorization::{
        self,
        password::{Password, PasswordRequirementError},
        Credentials, CredentialsError,
    },
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

/// Handler to change the password for an authorized user.
#[tracing::instrument(name = "Change password", skip(flash, data, user_service))]
pub async fn change_password(
    State(pool): State<Arc<PgPool>>,
    State(user_service): State<UserService>,
    flash: FlashMessage,
    user: AuthorizedUser,
    Form(data): Form<FormData>,
) -> Result<Response, ChangePasswordError> {
    if data.new_password.expose_secret() != data.new_password_check.expose_secret() {
        return Err(ChangePasswordError::NewPasswordNotMatching(flash));
    }

    let username = user_service
        .get_username(user.user_id())
        .await
        .context("Failed to retreive username")
        .map_err(ChangePasswordError::Unexpected)?;

    let credentials = Credentials::new(username, data.current_password);
    credentials
        .validate_credentials(&pool)
        .await
        .map_err(|e| match e {
            CredentialsError::InvalidPassword(_) => {
                ChangePasswordError::InvalidPassword(e, flash.clone())
            }
            _ => ChangePasswordError::Unexpected(anyhow::anyhow!(e)),
        })?;

    let password = Password::verify_password_requirements(data.new_password)
        .map_err(|es| ChangePasswordError::PasswordRequirementsNotSatisfied(es, flash.clone()))?;

    authorization::change_password(user.user_id(), password, &pool)
        .await
        .map_err(ChangePasswordError::Unexpected)?;

    Ok((
        flash.set_message("Your password has been changed.".to_string()),
        Redirect::to("/admin/password"),
    )
        .into_response())
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
    #[error("Password requirements not satisfied")]
    PasswordRequirementsNotSatisfied(Vec<PasswordRequirementError>, FlashMessage),
    #[error("New passwords does not match")]
    NewPasswordNotMatching(FlashMessage),
    #[error("Invalid password")]
    InvalidPassword(#[source] CredentialsError, FlashMessage),
}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> askama_axum::Response {
        tracing::error!("{self:?}");
        match self {
            Self::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::PasswordRequirementsNotSatisfied(missing_requirements, flash) => {
                let flash = flash.set_message_with_name(
                    "password_requirements",
                    missing_requirements
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                (flash, Redirect::to("/admin/password")).into_response()
            }
            Self::NewPasswordNotMatching(flash) => (
                flash.set_message(
                    "You entered two different new passwords - the field values must match."
                        .to_string(),
                ),
                Redirect::to("/admin/password"),
            )
                .into_response(),
            Self::InvalidPassword(_, flash) => (
                flash.set_message("The current password is incorrect.".to_string()),
                Redirect::to("/admin/password"),
            )
                .into_response(),
        }
    }
}
