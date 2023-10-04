use crate::require_login::AuthorizedUser;
use axum::{response::IntoResponse, Form};
use http::StatusCode;
use secrecy::Secret;

#[tracing::instrument(name = "Change password", skip(data))]
pub async fn change_password(
    user: AuthorizedUser,
    Form(data): Form<FormData>,
) -> Result<impl IntoResponse, ChangePasswordError> {
    Ok(StatusCode::OK)
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
