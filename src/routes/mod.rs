use axum::{headers::ContentType, response::IntoResponse, TypedHeader};
use utoipa::OpenApi;

pub mod admin;
pub mod health;
pub mod home;
pub mod login;
pub mod subscriptions;

#[derive(OpenApi)]
#[openapi(
    paths(health::is_alive, health::status, health::build_info),
    components(schemas(health::Status, health::BuildInfo))
)]
struct ApiDoc;

/// Endpoint to server openapi docs.
pub async fn serve_openapi_docs() -> impl IntoResponse {
    (
        TypedHeader(ContentType::json()),
        ApiDoc::openapi().to_json().unwrap(),
    )
        .into_response()
}
