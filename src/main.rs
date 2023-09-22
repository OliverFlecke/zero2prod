use std::io::stdout;
use zero2prod::{configuration::get_configuration, telemetry, App};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a tracing layer with the configured tracer
    let service_name = "zero2prod".to_string();
    let subscriber = telemetry::get_subscriber(service_name, stdout);
    let subscriber = telemetry::setup_optl(subscriber);

    telemetry::init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    App::build(configuration).await?.run_until_stopped().await?;

    Ok(())
}
