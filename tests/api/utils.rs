use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use derive_getters::Getters;
use http::StatusCode;
use once_cell::sync::Lazy;
use pretty_assertions::assert_eq;
use sqlx::PgPool;
use url::Url;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::get_configuration,
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
    test_user: TestUser,
    api_client: reqwest::Client,
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
    let db_pool = db::configure_database(config.database()).await;

    let app = App::build(config).await.expect("Failed to build app");
    let application_port = app.port();

    // Start server
    let _ = tokio::spawn(app.run_until_stopped());

    let address = format!("http://127.0.0.1:{application_port}");

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let app = TestApp {
        address,
        port: application_port,
        db_pool,
        email_server,
        test_user: TestUser::generate(),
        api_client,
    };

    app.test_user.store(app.db_pool()).await;

    app
}

#[derive(Getters)]
pub struct TestUser {
    user_id: Uuid,
    username: String,
    password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    /// Add a test user to the database.
    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test users");
    }
}

mod db {
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use zero2prod::configuration::DatabaseSettings;

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
}

pub mod client {
    use super::TestApp;

    /// Implemenation of a client for the services API.
    impl TestApp {
        fn at_url(&self, path: &str) -> String {
            format!("{}{path}", self.address())
        }

        /// Send a request to the health check endpoint.
        pub async fn health_check(&self) -> reqwest::Response {
            self.api_client()
                .get(self.at_url("/health"))
                .send()
                .await
                .expect("Failed to execute request.")
        }

        /// Send a POST request to the subscription endpoint.
        pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
            self.api_client()
                .post(self.at_url("/subscriptions"))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .expect("Failed to execute request.")
        }

        /// Send a POST request to the newsletter endpoint.
        pub async fn post_newsletter(&self, body: serde_json::Value) -> reqwest::Response {
            self.api_client()
                .post(self.at_url("/newsletters"))
                .json(&body)
                .basic_auth(
                    self.test_user().username(),
                    Some(self.test_user().password()),
                )
                .send()
                .await
                .expect("Failed to execute request")
        }

        /// Send a POST request to the `login` endpoint.
        pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
        where
            Body: serde::Serialize,
        {
            self.api_client()
                .post(self.at_url("/login"))
                .form(body)
                .send()
                .await
                .expect("Failed to execute request")
        }

        /// Get the HTML from the `/login` endpoint.
        pub async fn get_login_html(&self) -> String {
            self.api_client()
                .get(self.at_url("/login"))
                .send()
                .await
                .expect("Failed to execute request")
                .text()
                .await
                .unwrap()
        }

        pub async fn get_admin_dashboard(&self) -> reqwest::Response {
            self.api_client()
                .get(self.at_url("/admin/dashboard"))
                .send()
                .await
                .expect("Failed to execute request")
        }

        /// Get the HTML page from `/admin/dashboard` endpoint
        pub async fn get_admin_dashboard_html(&self) -> String {
            self.get_admin_dashboard().await.text().await.unwrap()
        }

        /// Send a request to get page to change user's password.
        pub async fn get_change_password(&self) -> reqwest::Response {
            self.api_client()
                .get(self.at_url("/admin/password"))
                .send()
                .await
                .expect("Failed to execute request")
        }

        /// Send a POST request to change user's password.
        pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
        where
            Body: serde::Serialize,
        {
            self.api_client()
                .post(self.at_url("/admin/password"))
                .form(body)
                .send()
                .await
                .expect("Failed to execute response")
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

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
