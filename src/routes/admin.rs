use self::{
    dashboard::admin_dashboard,
    password::{change_password, change_password_form},
};
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod dashboard;
pub(crate) mod password;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(admin_dashboard))
        .route("/password", get(change_password_form))
        .route("/password", post(change_password))
}
