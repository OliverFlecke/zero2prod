use crate::utils::spawn_app;
use axum::http::StatusCode;
use pretty_assertions::assert_eq;
use rstest::*;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await.expect("failed to create app");
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", app.address()))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("failed tot fetch saved subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
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
    let app = spawn_app().await.expect("failed to start app");
    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(&format!("{}/subscriptions", app.address()))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

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
    let app = spawn_app().await.expect("Failed to start app");
    let client = reqwest::Client::new();

    // Act
    let response = client
        .post(&format!("{}/subscriptions", app.address()))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        // Additional customised error message on test failure
        "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
        description
    );
}
