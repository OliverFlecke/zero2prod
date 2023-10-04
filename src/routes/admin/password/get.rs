use crate::{require_login::AuthorizedUser, service::flash_message::FlashMessage};
use askama::Template;
use axum::response::IntoResponse;

#[tracing::instrument(name = "Change password form", skip(flash))]
pub async fn change_password_form(flash: FlashMessage, user: AuthorizedUser) -> impl IntoResponse {
    ChangePasswordFormTemplate {
        error: flash.get_message(),
    }
}

#[derive(Template)]
#[template(path = "admin/change_password_form.html")]
struct ChangePasswordFormTemplate {
    error: Option<String>,
}
