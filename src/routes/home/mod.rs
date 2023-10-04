use crate::state::AppState;
use askama::Template;
use axum::{response::IntoResponse, routing::get, Router};

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(home_handler))
}

async fn home_handler() -> impl IntoResponse {
    HomeTemplate.into_response()
}

#[derive(Template, Default)]
#[template(path = "home.html")]
struct HomeTemplate;
