use crate::routes::*;
use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::{headers::ContentType, TypedHeader};
use http::{
    header::{self, ACCEPT},
    HeaderMap,
};
use utoipa::OpenApi;

/// Documentation for the service. Can be converted into JSON or YAML.
#[derive(OpenApi)]
#[openapi(
    paths(
        health::is_alive,
        health::status,
        health::build_info,
        home::home,
        login::get::login,
        login::post::login,
        subscriptions::subscribe,
        subscriptions::subscriptions_confirm::confirm,
        crate::metrics::metrics_endpoint,
    ),
    components(schemas(health::Status, health::BuildInfo))
)]
struct ApiDoc;

pub fn create_router() -> Router {
    Router::new()
        .route("/openapi", get(serve_openapi_docs))
        .route("/openapi.json", get(serve_openapi_docs_as_json))
        .route("/openapi.yaml", get(serve_openapi_docs_as_yaml))
}

/// Serve OpenApi docs based on the `Accept` header.
#[tracing::instrument(skip(headers))]
pub async fn serve_openapi_docs(headers: HeaderMap) -> impl IntoResponse {
    match headers.get(ACCEPT).and_then(|x| x.to_str().ok()) {
        Some("application/yaml") => serve_openapi_docs_as_yaml().await.into_response(),
        _ => serve_openapi_docs_as_json().await.into_response(),
    }
}

/// Endpoint to serve OpenApi docs as JSON.
#[tracing::instrument]
pub async fn serve_openapi_docs_as_json() -> impl IntoResponse {
    (
        TypedHeader(ContentType::json()),
        ApiDoc::openapi().to_json().unwrap(),
    )
}

/// Endpoint to serve OpenApi docs as YAML.
#[tracing::instrument]
pub async fn serve_openapi_docs_as_yaml() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/yaml")],
        ApiDoc::openapi().to_yaml().unwrap(),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn docs_can_be_converted_to_json_string() {
        assert!(ApiDoc::openapi().to_json().is_ok());
    }

    #[test]
    fn docs_can_be_converted_to_yaml_string() {
        assert!(ApiDoc::openapi().to_yaml().is_ok());
    }
}
