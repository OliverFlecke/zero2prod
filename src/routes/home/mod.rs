use crate::state::AppState;
use askama::Template;
use axum::{response::IntoResponse, routing::get, Router};

/// Create a router serve pages at the root of the service.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(home))
}

/// Serves the HTML for the home page.
#[tracing::instrument]
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = OK, description = "Home page for the service", content_type = "text/html")
    )
)]
async fn home() -> impl IntoResponse {
    HomeTemplate.into_response()
}

#[derive(Template, Default)]
#[template(path = "home.html")]
struct HomeTemplate;
