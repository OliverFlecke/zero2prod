use crate::require_login::AuthorizedUser;
use askama::Template;
use axum::response::IntoResponse;

/// Retreive the admin dashboard page.
#[tracing::instrument]
pub async fn admin_dashboard(user: AuthorizedUser) -> impl IntoResponse {
    let body = AdminDashboardTemplate {
        username: user.username().to_owned(),
    };

    body.into_response()
}

/// Template for HTML body of the admin portal.
#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboardTemplate {
    username: String,
}
