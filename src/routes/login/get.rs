use crate::service::flash_message::FlashMessage;
use askama::Template;
use axum::response::IntoResponse;

/// Return a view that renders a login form.
#[tracing::instrument(skip(flash))]
pub async fn login_form(flash: FlashMessage) -> impl IntoResponse {
    LoginTemplate {
        error: flash.get_message(),
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}
