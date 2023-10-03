use std::sync::Arc;

use anyhow::Context;
use askama::Template;
use axum::{
    body::Full,
    extract::State,
    response::{IntoResponse, Response},
};
use axum_sessions::extractors::ReadableSession;
use http::{header, StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

/// Retreive the admin dashboard page.
#[tracing::instrument(
    skip(session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn admin_dashboard(
    State(pool): State<Arc<PgPool>>,
    session: ReadableSession,
) -> Result<impl IntoResponse, AdminDashboardError> {
    let username = if let Some(user_id) = session.get::<Uuid>("user_id") {
        tracing::Span::current().record("user_id", &tracing::field::display(user_id));
        get_username(user_id, pool.as_ref())
            .await
            .map_err(AdminDashboardError::Unexpected)?
    } else {
        todo!()
    };

    tracing::Span::current().record("username", &tracing::field::display(&username));

    let body = AdminDashboardTemplate {
        username: username.to_string(),
    };

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .status(StatusCode::OK)
        .body(Full::from(body.render().unwrap()))
        .unwrap()
        .into_response();
    Ok(response)
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retreive a username")?;

    Ok(row.username)
}

#[derive(thiserror::Error)]
pub enum AdminDashboardError {
    #[error("Unexpected error")]
    Unexpected(#[source] anyhow::Error),
}

impl IntoResponse for AdminDashboardError {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

/// Template for HTML body of the admin portal.
#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboardTemplate {
    username: String,
}
