use crate::require_login::AuthorizedUser;
use askama::Template;
use axum::response::IntoResponse;

#[tracing::instrument(name = "Change password form")]
pub async fn change_password_form(user: AuthorizedUser) -> impl IntoResponse {
    ChangePasswordFormTemplate.into_response()
}

#[derive(Template)]
#[template(path = "admin/change_password_form.html")]
struct ChangePasswordFormTemplate;
