use crate::service::flash_message::FlashMessage;
use askama::Template;
use axum::response::IntoResponse;

/// Return a HTML page for a login form.
#[tracing::instrument(skip(flash))]
#[utoipa::path(
    get,
    path = "/login",
    responses(
        (status = OK, description = "Page for a user to login", content_type = "text/html")
    )
)]
pub async fn login(flash: FlashMessage) -> impl IntoResponse {
    LoginTemplate {
        error: flash.get_message(),
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}
