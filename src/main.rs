use sqlx::postgres::PgPoolOptions;
use std::{net::TcpListener, time::Duration};
use zero2prod::{configuration::get_configuration, telemetry, App};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let listener = TcpListener::bind(configuration.application().address())?;
    let pg_pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database().with_db());

    telemetry::init_subscriber(telemetry::get_subscriber(
        "zero2prod".to_string(),
        std::io::stdout,
    ));

    App::serve(listener, pg_pool).await?;

    Ok(())
}
