use crate::utils::{assert_is_redirect_to, spawn_app};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": Uuid::new_v4().to_string(),
        "password": Uuid::new_v4().to_string(),
    });
    let response = app.post_login(&login_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert!(flash_cookie.secure());
    assert!(flash_cookie.http_only());

    // Act - Part 2
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Act - Part 3
    sleep(Duration::from_secs(1)).await;
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"Authentication failed"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": app.test_user().username(),
        "password": app.test_user().password(),
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_admin_dashboard().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user().username())));
}
