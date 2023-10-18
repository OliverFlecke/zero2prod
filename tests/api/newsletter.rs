use std::time::Duration;

use self::utils::*;
use crate::utils::{assert_is_redirect_to, spawn_app};
use http::StatusCode;
use pretty_assertions::assert_eq;
use rstest::rstest;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.login_succesfully_with_mock_user()
        .await
        .error_for_status()
        .expect("Login failed");
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        // Assert no request is fired to email API.
        .expect(0)
        .mount(app.email_server())
        .await;

    // Act
    // A sketch of the newsletter payload structure. Might change later.
    let response = app.post_publish_newsletter(&full_body()).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.login_succesfully_with_mock_user()
        .await
        .error_for_status()
        .expect("Login failed");
    create_confirmed_subscriber(&app).await;

    // Mocking external email server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act
    _ = app.post_publish_newsletter(&full_body()).await;
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
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

    // Act - Part 1 - Login
    app.login_succesfully_with_mock_user()
        .await
        .error_for_status()
        .unwrap();

    // Act - Part 2 - Post body
    let response = app.post_publish_newsletter(&full_body()).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 3 - Follow redirect
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
    app.login_succesfully_with_mock_user()
        .await
        .error_for_status()
        .unwrap();

    // Act
    let response = app.post_publish_newsletter(&invalid_body).await;

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

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_succesfully_with_mock_user().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": "Newsletter body as plain text",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been published"));

    // Act - Part 3 - Submit newsletter form **again**
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 4 - Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been published"));

    // Mock verifies the newsletter has been sent exactly **once** on Drop.
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_succesfully_with_mock_user()
        .await
        .error_for_status()
        .expect("Failed to login user");

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act - Submit two newsletter forms concurrently
    let body = full_body();
    let response1 = app.post_publish_newsletter(&body);
    let response2 = app.post_publish_newsletter(&body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    // Mock verifies on Drop that we have sent the newsletter email **once**.
}

#[tokio::test]
async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries() {
    // Arrange
    let app = spawn_app().await;
    let newsletter_request_body = full_body();

    // Two subscribers instead of one
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;
    app.test_user().login(&app).await;

    // Part 1 - Submit newsletter form
    // Email delivery fails for second subscriber
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .up_to_n_times(1)
        .expect(1)
        .mount(app.email_server())
        .await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::INTERNAL_SERVER_ERROR))
        .up_to_n_times(1)
        .expect(1)
        .mount(app.email_server())
        .await;

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Part 2 - Retry submitting the form
    // Email delivery will suceed for both subscribers now
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .named("Delivery retry")
        .mount(app.email_server())
        .await;
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    // Mock verifies on Drop that we did not send duplicates.
}

mod utils {
    use crate::utils::{ConfirmationLinks, TestApp};
    use fake::{
        faker::{internet::en::SafeEmail, name::en::Name},
        Fake,
    };
    use http::StatusCode;
    use uuid::Uuid;
    use wiremock::{
        matchers::{method, path},
        Mock, MockBuilder, ResponseTemplate,
    };

    pub fn when_sending_an_email() -> MockBuilder {
        Mock::given(path("/email")).and(method("POST"))
    }

    /// Use the public API of the application under test to create an unconfirmed
    /// subscriber.
    pub async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
        let name: String = Name().fake();
        let email: String = SafeEmail().fake();
        let body = serde_urlencoded::to_string(&serde_json::json!({
            "name": name,
            "email": email,
        }))
        .unwrap();

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
    pub async fn create_confirmed_subscriber(app: &TestApp) {
        let confirmation_link = create_unconfirmed_subscriber(app).await;
        reqwest::get(confirmation_link.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    pub fn full_body() -> serde_json::Value {
        serde_json::json!({
            "title": "Newsletter title",
            "content": "Newsletter body as plain text",
            "idempotency_key": Uuid::new_v4().to_string(),
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

    #[tokio::test]
    async fn publish_newsletter_page_contains_idempotency_key() {
        // Arrange
        let app = spawn_app().await;
        app.login_succesfully_with_mock_user()
            .await
            .error_for_status()
            .expect("to succeed");

        // Act
        let html_page = app.get_newsletters_html().await;

        // Assert
        assert!(html_page.contains("<input hidden"));
    }
}
