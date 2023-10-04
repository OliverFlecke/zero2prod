use crate::{require_login::AuthorizedUser, service::flash_message::FlashMessage};
use axum::{
    response::{IntoResponse, Redirect, Response},
    Form,
};
use secrecy::{ExposeSecret, Secret};

#[tracing::instrument(name = "Change password", skip(flash, data))]
pub async fn change_password(
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

    todo!()
}

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum ChangePasswordError {}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> askama_axum::Response {
        todo!()
    }
}
