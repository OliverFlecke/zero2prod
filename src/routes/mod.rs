use crate::state::AppState;
use axum::Router;

pub mod admin;
pub mod health;
pub mod home;
pub mod login;
pub mod newsletters;
pub mod subscriptions;

pub fn build_router(app_state: &AppState) -> Router {
    Router::new()
        .nest("/health", health::create_router())
        .nest("/", home::create_router().with_state(app_state.clone()))
        .nest(
            "/login",
            login::create_router().with_state(app_state.clone()),
        )
        .nest(
            "/admin",
            admin::create_router().with_state(app_state.clone()),
        )
        .nest(
            "/subscriptions",
            subscriptions::create_router().with_state(app_state.clone()),
        )
        .nest(
            "/newsletters",
            newsletters::create_router().with_state(app_state.clone()),
        )
}
