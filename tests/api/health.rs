use crate::utils::spawn_app;
use axum::http::StatusCode;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await.expect("Failed to spawn our app.");

    // Act
    let response = app.health_check().await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(Some(0), response.content_length());
}
