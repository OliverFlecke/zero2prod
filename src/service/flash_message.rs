use crate::state::AppState;
use axum::{
    async_trait,
    body::Empty,
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, IntoResponseParts, Response},
};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;
use http::StatusCode;

const FLASH_MSG_KEY: &str = "_flash_";

// TODO: Consider adding message "levels" (e.g. error, info) to flash messages.

/// Service to send flash messages shown in the browser.
/// Note that this **MUST** be returned as part of the response.
#[derive(Clone)]
pub struct FlashMessage {
    cookie_jar: SignedCookieJar,
}

impl FlashMessage {
    /// Set a flash message that can be accessed in the next request to the server.
    /// TODO: Is this the right name for this? Maybe it should be `create` or `add`.
    pub fn set_message(self, message: String) -> Self {
        self.set_message_with_name("", message)
    }

    pub fn set_message_with_name(self, name: &str, message: String) -> Self {
        let cookie = Cookie::build(format!("{FLASH_MSG_KEY}{name}"), message)
            // Set the cookie to expire straight away so only the first
            // GET request will contain the error message.
            .max_age(cookie::time::Duration::seconds(1))
            .secure(true)
            .http_only(true)
            .path("/")
            .finish();
        let cookie_jar = self.cookie_jar.add(cookie);
        FlashMessage { cookie_jar }
    }

    /// Get the current flash message, if any.
    pub fn get_message(&self) -> Option<String> {
        self.get_message_with_name("")
    }

    pub fn get_message_with_name(&self, name: &str) -> Option<String> {
        self.cookie_jar
            .get(&format!("{FLASH_MSG_KEY}{name}"))
            .map(|c| c.value().to_string())
    }
}

/// Converts this into a response, as the cookie jar must be returned as part
/// of a handler for the messages to be send.
impl IntoResponseParts for FlashMessage {
    type Error = <SignedCookieJar as IntoResponseParts>::Error;

    fn into_response_parts(
        self,
        res: axum::response::ResponseParts,
    ) -> Result<axum::response::ResponseParts, Self::Error> {
        self.cookie_jar.into_response_parts(res)
    }
}

#[async_trait]
impl FromRequestParts<AppState> for FlashMessage {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        use axum::RequestPartsExt;
        let cookie_jar = parts
            .extract_with_state::<SignedCookieJar, AppState>(state)
            .await
            .map_err(|e| {
                tracing::error!("{e:?}");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Empty::default())
                    .unwrap()
                    .into_response()
            })?;

        Ok(FlashMessage { cookie_jar })
    }
}
