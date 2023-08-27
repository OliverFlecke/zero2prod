use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let listener = TcpListener::bind(format!("0.0.0.0:{}", configuration.application_port()))?;
    let pg_pool = PgPool::connect(&configuration.database().connection_string())
        .await
        .expect("Failed to connect to Postgres");

    zero2prod::App::serve(listener, pg_pool).await?;

    Ok(())
}
