use crate::state::HmacSecret;
use askama::Template;
use axum::{
    body::Full,
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::SignedCookieJar;
use http::StatusCode;
use std::sync::Arc;

/// Return a view that renders a login form.
#[tracing::instrument()]
pub async fn login_form(
    State(hmac_secret): State<Arc<HmacSecret>>,
    cookie_jar: SignedCookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    let body = LoginTemplate {
        error: cookie_jar.get("_flash").map(|c| c.value().to_string()),
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Full::from(body.render().unwrap()))
        .unwrap();

    Ok(response)
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}
