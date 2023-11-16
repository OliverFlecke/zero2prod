use http::{
    header::{ACCEPT, CONTENT_TYPE},
    StatusCode,
};
use rstest::rstest;

use crate::utils::spawn_app;

#[tokio::test]
async fn open_api_documentation_can_be_retrieved_as_json() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client()
        .get(app.at_url("/docs/openapi.json"))
        .send()
        .await
        .expect("Request failed");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(response.content_length(), Some(0));
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|x| x.to_str().ok()),
        Some("application/json")
    );
}

#[tokio::test]
async fn open_api_documentation_can_be_retrieved_as_yaml() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client()
        .get(app.at_url("/docs/openapi.yaml"))
        .send()
        .await
        .expect("Request failed");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(response.content_length(), Some(0));
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|x| x.to_str().ok()),
        Some("application/yaml")
    );
}

#[rstest]
#[case("json")]
#[case("yaml")]
#[tokio::test]
async fn open_api_documentation_can_be_retrieved(#[case] content_type: String) {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client()
        .get(app.at_url("/docs/openapi"))
        .header(ACCEPT, format!("application/{content_type}"))
        .send()
        .await
        .expect("Request failed");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    assert_ne!(response.content_length(), Some(0));
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|x| x.to_str().ok()),
        Some(format!("application/{content_type}").as_str())
    );
}
