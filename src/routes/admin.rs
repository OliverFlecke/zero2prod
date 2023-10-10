use self::{
    dashboard::admin_dashboard,
    logout::log_out,
    newsletters::publish_newsletter,
    password::{change_password, change_password_form},
};
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod dashboard;
mod logout;
pub(crate) mod newsletters;
pub(crate) mod password;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(admin_dashboard))
        .route("/password", get(change_password_form))
        .route("/password", post(change_password))
        .route("/logout", post(log_out))
        .route("/newsletters", post(publish_newsletter))
}
