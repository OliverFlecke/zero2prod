use crate::state::AppState;
use axum::{
    body::Full,
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http::{header, StatusCode};
use std::sync::Arc;
use tera::Tera;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(home_handler))
}

#[axum::debug_handler]
async fn home_handler(State(templates): State<Arc<Tera>>) -> Response {
    let body = templates
        .render("home.html", &tera::Context::default())
        .unwrap();

    Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .status(StatusCode::OK)
        .body(Full::from(body))
        .unwrap()
        .into_response()
}
