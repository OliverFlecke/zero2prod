use std::{
    fmt::{Debug, Display},
    io::stdout,
};
use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration, issue_delivery_worker::run_worker_until_stopped, telemetry,
    App,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a tracing layer with the configured tracer
    let service_name = "zero2prod".to_string();
    let configuration = get_configuration().expect("Failed to read configuration.");

    let subscriber = telemetry::get_subscriber(service_name, stdout);
    if *configuration.application().open_telemetry() {
        let subscriber = telemetry::setup_optl(subscriber);
        telemetry::init_subscriber(subscriber);
    } else {
        telemetry::init_subscriber(subscriber);
    }

    tracing::debug!("{:#?}", configuration);

    let application = App::build(configuration.clone()).await?;

    if *configuration.application().enable_background_worker() {
        let application_task = tokio::spawn(application.run_until_stopped());
        let worker_task = tokio::spawn(run_worker_until_stopped(configuration));

        tokio::select! {
            o = application_task => report_exit("API", o),
            o = worker_task => report_exit("Background worker", o),
        };
    } else {
        application.run_until_stopped().await.unwrap();
    }

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{}' task failed to complete",
                task_name
            )
        }
    }
}
