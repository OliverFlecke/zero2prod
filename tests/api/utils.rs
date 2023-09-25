use derive_getters::Getters;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use url::Url;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber},
    App,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), std::io::sink);
        init_subscriber(subscriber);
    };
});

#[derive(Getters)]
pub struct TestApp {
    address: String,
    port: u16,
    db_pool: PgPool,
    email_server: MockServer,
}

/// Spawn a instance of the app on a random port.
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let config = {
        let mut c = get_configuration().expect("Failed to read configuration");

        // Generate a unique name for each DB
        c.database.name = Uuid::new_v4().to_string();
        // Make OS choose random port
        c.application.port = 0;
        // Use the mock server as the email server API
        c.email_client.base_url = email_server.uri();

        c
    };

    // Setup database
    let db_pool = configure_database(config.database()).await;

    let app = App::build(config).await.expect("Failed to build app");
    let application_port = app.port();

    // Start server
    let _ = tokio::spawn(app.run_until_stopped());

    let address = format!("http://127.0.0.1:{application_port}");
    TestApp {
        address,
        port: application_port,
        db_pool,
        email_server,
    }
}

/// Configure database for testing. This will ensure a database is created
/// with the given db name from the config and that all migrations are applied.
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.name()).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate the database
    let db_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    db_pool
}

pub mod client {
    use super::TestApp;
    use reqwest::Client;
    use uuid::Uuid;

    /// Implemenation of a client for the services API.
    impl TestApp {
        /// Send a request to the health check endpoint.
        pub async fn health_check(&self) -> reqwest::Response {
            Client::new()
                .get(format!("{}/health", self.address))
                .send()
                .await
                .expect("Failed to execute request.")
        }

        /// Send a POST request to the subscription endpoint.
        pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
            Client::new()
                .post(&format!("{}/subscriptions", self.address))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .expect("Failed to execute request.")
        }

        /// Send a POST request to the newsletter endpoint.
        pub async fn post_newsletter(&self, body: serde_json::Value) -> reqwest::Response {
            reqwest::Client::new()
                .post(&format!("{}/newsletters", self.address()))
                .json(&body)
                .basic_auth(Uuid::new_v4().to_string(), Some(Uuid::new_v4().to_string()))
                .send()
                .await
                .expect("Failed to execute request")
        }
    }
}

pub mod mock {
    use super::TestApp;
    use http::StatusCode;
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    /// Utilitize to help mocking the email API.
    impl TestApp {
        pub async fn mock_send_email_endpoint_to_ok(&self) {
            Mock::given(path("/email"))
                .and(method("POST"))
                .respond_with(ResponseTemplate::new(StatusCode::OK))
                .mount(self.email_server())
                .await;
        }
    }
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    /// Extract the confirmation links from an email request received through
    /// the wiremock.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // Extract link from request fields
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            confirmation_link.set_port(Some(*self.port())).unwrap();
            // Verify link is pointing to localhost
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link
        };

        ConfirmationLinks {
            html: get_link(&body["HtmlBody"].as_str().unwrap()),
            plain_text: get_link(&body["TextBody"].as_str().unwrap()),
        }
    }
}
