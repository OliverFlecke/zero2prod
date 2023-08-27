use axum::{http::StatusCode, routing::post, Form, Router};

#[derive(Debug, serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

/// Create a router to serve endpoints.
pub fn create_router() -> Router {
    Router::new().route("/", post(subscribe))
}

async fn subscribe(Form(_data): Form<FormData>) -> StatusCode {
    StatusCode::OK
}
