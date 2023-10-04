use crate::{service::flash_message::FlashMessage, state::HmacSecret};
use askama::Template;
use axum::{extract::State, response::IntoResponse};
use std::sync::Arc;

/// Return a view that renders a login form.
#[tracing::instrument(skip(flash_message))]
pub async fn login_form(
    State(hmac_secret): State<Arc<HmacSecret>>,
    flash_message: FlashMessage,
) -> impl IntoResponse {
    LoginTemplate {
        error: flash_message.get_message(),
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}
