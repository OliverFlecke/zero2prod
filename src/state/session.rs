use axum::{
    async_trait,
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use http::request::Parts;
use uuid::Uuid;

const USER_ID_KEY: &str = "user_id";

pub struct Session(tower_sessions::Session);

impl Session {
    /// Regenerate the current session for the user.
    pub fn regenerate(&mut self) {
        self.0.clear();
    }

    /// Log the user out of the current session.
    pub fn log_out(self) {
        tracing::debug!("Clearing session for user");
        self.0.delete();
    }

    pub fn insert_user_id(&mut self, user_id: Uuid) -> anyhow::Result<()> {
        tracing::debug!("Inserting user id for {}", user_id);
        self.0
            .insert(USER_ID_KEY, user_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        self.0.get::<Uuid>(USER_ID_KEY).ok().flatten()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Session {
    type Rejection = TypedSessionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        tracing::trace!("Extracting session from request");
        use axum::RequestPartsExt;
        let session = parts
            .extract::<tower_sessions::Session>()
            .await
            .map_err(|(_, e)| TypedSessionError::UnknownError(anyhow::anyhow!(e)))?;

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
