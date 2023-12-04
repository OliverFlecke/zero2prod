use crate::state::{session::Session, AppState};
use axum::{
    async_trait,
    body::Body,
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use derive_getters::Getters;
use http::StatusCode;
use uuid::Uuid;

/// Represents a session where the user is successfully logged in.
#[derive(Debug, Getters)]
pub struct AuthorizedUser {
    user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthorizedUser {
    type Rejection = AuthorizedUserError;

    #[tracing::instrument(
        skip(parts, _state),
        fields(user_id=tracing::field::Empty)
    )]
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
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

        Ok(AuthorizedUser { user_id })
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
                    .body(Body::empty())
                    .unwrap()
                    .into_response()
            }
            Self::NotLoggedIn => Redirect::to("/login").into_response(),
        }
    }
}
