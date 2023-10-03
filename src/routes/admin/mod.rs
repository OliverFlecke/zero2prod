use self::dashboard::admin_dashboard;
use crate::state::AppState;
use axum::{routing::get, Router};

pub mod dashboard;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/dashboard", get(admin_dashboard))
}
