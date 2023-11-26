use anyhow::Context;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http::StatusCode;
use prometheus::{Encoder, Gauge, Registry, TextEncoder};
use std::{sync::Arc, time::Duration};
use systemstat::{Platform, System};

pub fn build_metric_layers(router: Router) -> anyhow::Result<Router> {
    let registry = Arc::new(Registry::new());
    spawn_metrics_monitor_task_for_cpu_and_memory(&registry)?;

    let router = router.route("/metrics", get(metrics_endpoint).with_state(registry));

    Ok(router)
}

#[tracing::instrument(skip(registry))]
async fn metrics_endpoint(State(registry): State<Arc<Registry>>) -> Result<String, MetricsError> {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder
        .encode(&metric_families, &mut buffer)
        .context("Failed to encode metrics")
        .map_err(MetricsError::UnexpectedError)?;

    // Output to the standard output.
    String::from_utf8(buffer)
        .context("Failed to convert metrics to a valid string")
        .map_err(MetricsError::UnexpectedError)
}

#[derive(thiserror::Error)]
pub enum MetricsError {
    #[error("Unexpected error when generating metrics")]
    UnexpectedError(#[source] anyhow::Error),
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

/// Add CPU and memory usage metrics to the `Registry`.
/// Spawns a task on the tokio runtime to continuesly monitor the CPU and
/// memory usage of the application.
fn spawn_metrics_monitor_task_for_cpu_and_memory(registry: &Registry) -> anyhow::Result<()> {
    let cpu_usage_percentage = Gauge::new("cpu_usage_percentage", "Current CPU usage in percent")
        .context("Failed to create `cpu_usage_percentage` gauge")?;
    registry
        .register(Box::new(cpu_usage_percentage.clone()))
        .context("Failed to register `cpu_usage` metric")?;

    let mem_usage_percentage =
        Gauge::new("mem_usage_percentage", "Current memory usage in percent")
            .context("Failed to create memory gauge")?;
    registry
        .register(Box::new(mem_usage_percentage.clone()))
        .context("Failed to register `mem_usage_percentage` metric")?;

    let mem_usage_total = Gauge::new("mem_usage_total", "Current memory usage in absoulte value")
        .context("Failed to create memory gauge for absolute value")?;
    registry
        .register(Box::new(mem_usage_total.clone()))
        .context("Failed to register `mem_usage_total` metric")?;

    // Spawn task on the tokio runtime to continuesly monitor.
    tokio::spawn(async move {
        let sys = System::new();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            match sys.cpu_load_aggregate().and_then(|cpu| cpu.done()) {
                Ok(cpu) => {
                    cpu_usage_percentage.set(f64::trunc(
                        ((cpu.system * 100.0) + (cpu.user * 100.0)).into(),
                    ));
                }
                Err(e) => tracing::error!("Failed to load CPU usage: {}", e),
            }
            match sys.memory() {
                Ok(mem) => {
                    let memory_used = mem.total.as_u64() - mem.free.as_u64();
                    mem_usage_total.set(f64::trunc(memory_used as f64));

                    let percentage_used = (memory_used as f64 / mem.total.as_u64() as f64) * 100.0;
                    mem_usage_percentage.set(f64::trunc(percentage_used));
                }
                Err(e) => tracing::error!("Failed to load memory usage: {}", e),
            }
        }
    });

    Ok(())
}
