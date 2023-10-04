use crate::{require_login::AuthorizedUser, service::user::UserService};
use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use http::StatusCode;

/// Retreive the admin dashboard page.
#[tracing::instrument(name = "Admin dashboard", skip(user_service))]
pub async fn admin_dashboard(
    State(user_service): State<UserService>,
    user: AuthorizedUser,
) -> Result<impl IntoResponse, Response> {
    let username = user_service
        .get_username(user.user_id())
        .await
        .map_err(|e| {
            tracing::error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    let body = AdminDashboardTemplate { username };

    Ok(body.into_response())
}

/// Template for HTML body of the admin portal.
#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboardTemplate {
    username: String,
}
