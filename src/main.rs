use std::io::stdout;
use zero2prod::{configuration::get_configuration, telemetry, App};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_subscriber(telemetry::get_subscriber("zero2prod".to_string(), stdout));

    let configuration = get_configuration().expect("Failed to read configuration.");
    App::build(configuration).await?.run_until_stopped().await?;

    Ok(())
}
