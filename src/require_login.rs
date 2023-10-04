use crate::{
    service::user::get_username,
    state::{session::Session, AppState},
};
use axum::{
    async_trait,
    body::Empty,
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use derive_getters::Getters;
use http::StatusCode;
use uuid::Uuid;

/// Represents a session where the user is successfully logged in.
#[derive(Getters)]
pub struct AuthorizedUser {
    user_id: Uuid,
    username: String,
}

impl std::fmt::Debug for AuthorizedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizedUser")
            .field("user_id", self.user_id())
            .field("username", self.username())
            .finish()
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthorizedUser {
    type Rejection = AuthorizedUserError;

    #[tracing::instrument(
        skip(parts, state),
        fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
    )]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        use axum::RequestPartsExt;
        let session = parts
            .extract::<Session>()
            .await
            .map_err(|e| AuthorizedUserError::Unexpected(anyhow::anyhow!(e)))?;

        let Some(user_id) = session.get_user_id() else {
            return Err(AuthorizedUserError::NotLoggedIn);
        };
        tracing::Span::current().record("user_id", &tracing::field::display(user_id));

        let username = get_username(user_id, state.db_pool())
            .await
            .map_err(AuthorizedUserError::Unexpected)?;
        tracing::Span::current().record("username", &tracing::field::display(&username));

        Ok(AuthorizedUser { user_id, username })
    }
}

#[derive(thiserror::Error)]
pub enum AuthorizedUserError {
    #[error("Unexpected error")]
    Unexpected(#[source] anyhow::Error),
    #[error("User not logged in")]
    NotLoggedIn,
}

impl IntoResponse for AuthorizedUserError {
    fn into_response(self) -> Response {
        match self {
            Self::Unexpected(e) => {
                tracing::error!("{e:?}");

                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Empty::default())
                    .unwrap()
                    .into_response()
            }
            Self::NotLoggedIn => Redirect::to("/login").into_response(),
        }
    }
}
