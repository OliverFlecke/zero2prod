use hyper::StatusCode;

mod common;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = common::spawn_app().await.expect("Failed to spawn our app.");
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health", app.address()))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(Some(0), response.content_length());
}
