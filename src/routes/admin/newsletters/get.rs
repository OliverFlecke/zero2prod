use askama::Template;
use axum::response::IntoResponse;

/// Returns a HTML page with a form to publish a new newsletter.
#[tracing::instrument(name = "Publish newsletter page")]
pub async fn publish_newsletter_html() -> impl IntoResponse {
    PublishNewsletter
}

#[derive(Template)]
#[template(path = "admin/publish_newsletter.html")]
pub struct PublishNewsletter;
