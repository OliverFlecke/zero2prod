use derive_getters::Getters;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
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

#[derive(Debug, Getters)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spawn a instance of the app on a random port.
pub async fn spawn_app() -> anyhow::Result<TestApp> {
    Lazy::force(&TRACING);
    let config = {
        let mut c = get_configuration().expect("Failed to read configuration");

        // Generate a unique name for each DB.
        c.database.name = Uuid::new_v4().to_string();
        // Make OS choose random port
        c.application.port = 0;

        c
    };

    // Setup database
    let db_pool = configure_database(config.database()).await;

    let app = App::build(config)?;
    let application_port = app.port();

    // Start server
    let _ = tokio::spawn(app.run_until_stopped());

    let address = format!("http://127.0.0.1:{application_port}");
    Ok(TestApp { address, db_pool })
}

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
