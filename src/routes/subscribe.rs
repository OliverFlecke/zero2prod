use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Form, Router};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

/// Create a router to serve endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/", post(subscribe))
}

/// Subscribe to the newsletter with an email and name.
async fn subscribe(
    State(connection): State<Arc<PgPool>>,
    Form(data): Form<FormData>,
) -> StatusCode {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %data.email,
        subscriber_name = %data.name,
    );
    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!("Saving new subscriber details to db");
    match sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at)
           VALUES($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        data.email,
        data.name,
        Utc::now()
    )
    .execute(connection.as_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("New subscriber details have been saved");
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("Failed to execute query: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
