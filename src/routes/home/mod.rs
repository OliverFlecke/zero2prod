use crate::state::AppState;
use askama::Template;
use axum::{
    body::Full,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http::{header, StatusCode};

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(home_handler))
}

async fn home_handler() -> Result<Response, StatusCode> {
    let body = HomeTemplate::default();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .status(StatusCode::OK)
        .body(Full::from(body.render().unwrap()))
        .unwrap()
        .into_response())
}

#[derive(Template, Default)]
#[template(path = "home.html")]
struct HomeTemplate;
