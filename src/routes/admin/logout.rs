use crate::{
    service::flash_message::FlashMessage,
    state::{session::Session, AppState},
};
use axum::response::{IntoResponse, Redirect};

/// Log the user out of the current session.
#[tracing::instrument(name = "Log out", skip(session, flash))]
#[axum::debug_handler(state = AppState)]
pub async fn log_out(flash: FlashMessage, session: Session) -> impl IntoResponse {
    session.log_out();
    let flash = flash.set_message("You have successfully logged out.".to_string());

    (flash, Redirect::to("/login")).into_response()
}
