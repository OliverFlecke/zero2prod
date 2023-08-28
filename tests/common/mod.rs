use derive_getters::Getters;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber},
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

#[derive(Debug, Getters)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spawn a instance of the app on a random port.
pub async fn spawn_app() -> anyhow::Result<TestApp> {
    Lazy::force(&TRACING);
    let mut configuration = get_configuration().expect("Failed to read configuration");

    // Setup database
    configuration.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(configuration.database()).await;

    // Create listener
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let address = format!("http://{}", listener.local_addr().unwrap());

    // Start server
    let server = zero2prod::App::serve(listener, db_pool.clone());
    let _ = tokio::spawn(server);

    Ok(TestApp { address, db_pool })
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name()).as_str())
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
