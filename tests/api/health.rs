use crate::utils::spawn_app;
use axum::http::StatusCode;
use chrono::NaiveDateTime;
use pretty_assertions::assert_eq;
use serde_json::Value;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.health_check().await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK.as_u16());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn info_endpoint_gives_build_info() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client()
        .get(app.at_url("/info"))
        .send()
        .await
        .expect("Request failed");

    // Assert
    assert_eq!(response.status(), StatusCode::OK.as_u16());

    let body = response.text().await.expect("unable to read body");
    let body: Value = serde_json::from_str(&body).expect("unable to parse json");
    assert!(body
        .get("version")
        .and_then(|x| x.as_str())
        .filter(|x| !x.is_empty())
        .is_some());
    assert!(body
        .get("build")
        .and_then(|x| x.as_str())
        .filter(|x| !x.is_empty())
        .is_some());
    assert!(body
        .get("build_timestamp")
        .and_then(|x| x.as_str())
        .and_then(|x| NaiveDateTime::parse_from_str(x, "%Y-%m-%dT%H:%M:%S%.f").ok())
        .is_some());
}

#[tokio::test]
async fn status_endpoint_returns_up_when_both_services_are_up() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .api_client()
        .get(app.at_url("/status"))
        .send()
        .await
        .expect("Request failed");

    // Assert
    assert_eq!(response.status(), StatusCode::OK.as_u16());

    let body = response.text().await.expect("unable to read body");
    let body: Value = serde_json::from_str(&body).expect("unable to parse json");
    assert_eq!(
        body.get("is_db_connected")
            .and_then(|x| x.as_bool())
            .unwrap(),
        true
    );
    assert_eq!(
        body.get("is_redis_connected")
            .and_then(|x| x.as_bool())
            .unwrap(),
        true
    );
}
