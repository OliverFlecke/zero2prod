pub mod get;
pub mod post;

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get::login))
        .route("/", post(post::login))
}
