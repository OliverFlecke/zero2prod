use axum::http::StatusCode;
use rstest::*;
use sqlx::{Connection, PgConnection};
use zero2prod::configuration::get_configuration;

mod common;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = common::spawn_app().await.expect("failed to create app");
    let configuration = get_configuration().expect("failed to read configuration");
    let mut pg_connection = PgConnection::connect(&configuration.database().connection_string())
        .await
        .expect("failed to connect to Postgres");
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
        .fetch_one(&mut pg_connection)
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
async fn subscribe_returns_a_400_when_data_is_missing(
    #[case] body: String,
    #[case] error_message: String,
) {
    // Arrange
    let app = common::spawn_app().await.expect("failed to start app");
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
        "The API did not fail with 400 Bad Request when the payload was {}.",
        error_message
    );
}
