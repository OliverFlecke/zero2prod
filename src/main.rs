use std::net::TcpListener;
use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let listener = TcpListener::bind(format!("0.0.0.0:{}", configuration.application_port()))?;

    zero2prod::App::create().serve(listener).await?;

    Ok(())
}
