use askama::Template;
use axum::response::IntoResponse;

use crate::service::flash_message::FlashMessage;

/// Returns a HTML page with a form to publish a new newsletter.
#[tracing::instrument(name = "Publish newsletter page", skip(flash))]
pub async fn publish_newsletter_html(flash: FlashMessage) -> impl IntoResponse {
    PublishNewsletter {
        message: flash.get_message(),
    }
}

#[derive(Template)]
#[template(path = "admin/publish_newsletter.html")]
pub struct PublishNewsletter {
    message: Option<String>,
}
