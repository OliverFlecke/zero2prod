use crate::utils::{spawn_app, ConfirmationLinks, TestApp};
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
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = app.post_newsletter(newsletter_request_body).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = app.post_newsletter(newsletter_request_body).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[rstest]
#[case(serde_json::json!({
    "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    }
}), "missing title")]
#[case(serde_json::json!({"title": "Newsletter!" }), "missing content")]
#[case(serde_json::json!({
    "title": "Newsletter!",
    "content": {
        "text": "Newsletter body as plain text",
    }
}), "missing html content")]
#[case(serde_json::json!({
    "title": "Newsletter!",
    "content": {
        "html": "Newsletter body as plain text",
    }
}), "missing text content")]
#[tokio::test]
async fn newsletters_returns_422_for_invalid_data(
    #[case] invalid_body: serde_json::Value,
    #[case] error_message: String,
) {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.post_newsletter(invalid_body).await;

    // Assert
    assert_eq!(
        StatusCode::UNPROCESSABLE_ENTITY,
        response.status(),
        "The API did not fail with 422 Unprocessable entity when payload was {}.",
        error_message
    )
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    // Arrange
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", app.address()))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
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
