use crate::state::HmacSecret;
use askama::Template;
use axum::{
    body::Full,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use http::StatusCode;
use secrecy::ExposeSecret;
use std::sync::Arc;

/// Return a view that renders a login form.
#[tracing::instrument(skip(hmac_secret))]
pub async fn login_form(
    State(hmac_secret): State<Arc<HmacSecret>>,
    params: Option<Query<QueryParams>>,
) -> Result<impl IntoResponse, StatusCode> {
    let body = LoginTemplate {
        error: params.and_then(|x| {
            x.0.verify(hmac_secret.as_ref())
                .map_err(|e| {
                    tracing::warn!(
                        error.message = %e,
                        error.cause_chain = ?e,
                        "Failed to verify query parameters using the HMAC tag"
                    );
                    e
                })
                .ok()
        }),
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Full::from(body.render().unwrap()))
        .unwrap();

    Ok(response)
}

#[derive(Debug, serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        tracing::debug!("Verifying hmac: {}", self.tag);

        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            Hmac::<sha3::Sha3_256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}
