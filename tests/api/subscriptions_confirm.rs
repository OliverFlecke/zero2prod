//! Integration test for confirmation of subscription to the newsletter.
use crate::utils::spawn_app;
use http::StatusCode;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn confirmations_without_tokens_are_rejected_with_a_400() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address()))
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.mock_send_email_endpoint_to_ok().await;
    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server().received_requests().await.unwrap()[0];
    let confirmation_link = app.get_confirmation_links(email_request);

    // Act
    let response = reqwest::get(confirmation_link.html).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.mock_send_email_endpoint_to_ok().await;
    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server().received_requests().await.unwrap()[0];
    let confirmation_link = app.get_confirmation_links(email_request);

    // Act
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(app.db_pool())
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
