use crate::{require_login::AuthorizedUser, service::flash_message::FlashMessage};
use askama::Template;
use axum::response::IntoResponse;

#[tracing::instrument(name = "Change password form", skip(flash))]
pub async fn change_password_form(flash: FlashMessage, user: AuthorizedUser) -> impl IntoResponse {
    ChangePasswordFormTemplate {
        error: flash.get_message(),
        password_requirements: flash
            .get_message_with_name("password_requirements")
            .map(|x| x.split(',').map(String::from).collect()),
    }
}

#[derive(Template)]
#[template(path = "admin/change_password_form.html")]
struct ChangePasswordFormTemplate {
    error: Option<String>,
    password_requirements: Option<Vec<String>>,
}
