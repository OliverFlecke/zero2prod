use axum::{
    async_trait,
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use axum_sessions::extractors::WritableSession;
use http::request::Parts;
use uuid::Uuid;

const USER_ID_KEY: &str = "user_id";

pub struct Session(WritableSession);

impl Session {
    /// Regenerate the current session for the user. See `WritableSession` for more details.
    pub fn regenerate(&mut self) {
        self.0.regenerate();
    }

    /// Log the user out of the current session.
    pub fn log_out(mut self) {
        self.0.destroy()
    }

    // TODO: Use custom errors instead of `anyhow`
    pub fn insert_user_id(&mut self, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.0
            .insert(USER_ID_KEY, user_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        self.0.get(USER_ID_KEY)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Session {
    type Rejection = TypedSessionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        tracing::trace!("Extracting session from request");
        use axum::RequestPartsExt;
        let session = parts
            .extract::<WritableSession>()
            .await
            .map_err(|e| TypedSessionError::UnknownError(anyhow::anyhow!(e)))?;

        Ok(Self(session))
    }
}

#[derive(thiserror::Error)]
pub enum TypedSessionError {
    #[error("{0}")]
    UnknownError(#[source] anyhow::Error),
}

impl IntoResponse for TypedSessionError {
    fn into_response(self) -> Response {
        http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
