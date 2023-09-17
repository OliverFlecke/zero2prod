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
