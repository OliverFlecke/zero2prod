use self::utils::*;
use crate::utils::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use http::StatusCode;
use pretty_assertions::assert_eq;
use rstest::rstest;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        // Assert no request is fired to email API.
        .expect(0)
        .mount(app.email_server())
        .await;

    // Act
    // A sketch of the newsletter payload structure. Might change later.
    let response = app.post_newsletter(&full_body()).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // Mocking external email server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act
    _ = app.post_newsletter(&full_body()).await;
}

#[tokio::test]
async fn request_is_redirected_to_publish_page_after_success_and_shows_message() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // Mocking external email server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act - Part 1 - Post body
    let response = app.post_newsletter(&full_body()).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been published"));
}

#[rstest]
#[case(serde_json::json!({
    "content": "Newsletter body as plain text",
}), "missing title")]
#[case(serde_json::json!({"title": "Newsletter!" }), "missing content")]
#[tokio::test]
async fn newsletters_returns_422_for_invalid_data(
    #[case] invalid_body: serde_json::Value,
    #[case] error_message: String,
) {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.post_newsletter(&invalid_body).await;

    // Assert
    assert_eq!(
        StatusCode::UNPROCESSABLE_ENTITY,
        response.status(),
        "The API did not fail with 422 Unprocessable entity when payload was {}.",
        error_message
    )
}

#[tokio::test]
async fn requests_missing_authorization_is_redirected_to_login() {
    // Arrange
    let app = spawn_app().await;

    let response = app
        .api_client()
        .post(app.at_url("/admin/newsletters"))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_is_redirect_to(&response, "/login");
}

// Utils

/// Use the public API of the application under test to create an unconfirmed
/// subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(app.email_server())
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    // Get confirmation links
    let email_request = app
        .email_server()
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

/// Create a confirmed subscriber.
async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

mod utils {
    pub fn full_body() -> serde_json::Value {
        serde_json::json!({
            "title": "Newsletter title",
            "content": "Newsletter body as plain text",
        })
    }
}

mod get {
    use crate::utils::{assert_is_redirect_to, spawn_app};

    #[tokio::test]
    async fn request_for_publish_page_without_authorized_user_redirects_to_login() {
        // Arrange
        let app = spawn_app().await;

        // Act
        let response = app.get_newsletters().await;

        // Assert
        assert_is_redirect_to(&response, "/login");
    }

    #[tokio::test]
    async fn authorized_request_returns_html_form() {
        // Arrange
        let app = spawn_app().await;
        app.login_succesfully_with_mock_user()
            .await
            .error_for_status()
            .expect("to succeed");

        // Act
        let html_page = app.get_newsletters_html().await;

        // Assert
        assert!(html_page.contains("<form"));
    }
}
