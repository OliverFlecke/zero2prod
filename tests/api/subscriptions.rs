use crate::utils::spawn_app;
use axum::http::StatusCode;
use pretty_assertions::assert_eq;
use rstest::*;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    app.mock_send_email_endpoint_to_ok().await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // Arrange
    let app = spawn_app().await;
    app.mock_send_email_endpoint_to_ok().await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.post_subscriptions(body.into()).await;

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(app.db_pool())
        .await
        .expect("failed tot fetch saved subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[rstest]
#[case("name=le%20guin", "missing the email")]
#[case("email=ursula_le_guin%40gmail.com", "missing the name")]
#[case("", "missing both name and email")]
#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing(
    #[case] body: String,
    #[case] error_message: String,
) {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        // Additional customised error message on test failure
        "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
        error_message
    );
}

#[rstest]
#[case("name=&email=ursula_le_guin%40gmail.com", "empty name")]
#[case("name=Ursula&email=", "empty email")]
#[case("name=Ursula&email=definitely-not-a-valid-email", "invalid email")]
#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_empty(
    #[case] body: String,
    #[case] description: String,
) {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        // Additional customised error message on test failure
        "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
        description
    );
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(app.email_server())
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    // Mock will do asserts on drop.
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(app.email_server())
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &app.email_server().received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(app.db_pool())
        .await
        .unwrap();

    // Act
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
