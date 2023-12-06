use anyhow::Context;
use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http::StatusCode;
use lazy_static::lazy_static;
use prometheus::{
    register_gauge, register_histogram_vec, register_int_counter_vec, Encoder, Gauge, HistogramVec,
    IntCounterVec, TextEncoder,
};

lazy_static! {
    static ref REQUEST_COUNTER: IntCounterVec = register_int_counter_vec!(
        "request_count",
        "Number of requests received",
        &["path", "http_method"]
    )
    .unwrap();
    /// Counts the number of active request. Leaving this as a pure counter for now.
    static ref REQUEST_ACTIVE_GAUGE: Gauge =
        register_gauge!("request_active_count", "Number of active requests").unwrap();
    static ref REQUEST_DURATION: HistogramVec =
        register_histogram_vec!("request_duration", "Duration of requests", &["path", "http_method"]).unwrap();
    static ref RESPONSE_COUNTER: IntCounterVec = register_int_counter_vec!(
        "response_code_count",
        "Responses by status code",
        &["path", "http_method", "code"]
    )
    .unwrap();
}

/// Configure layers and routes for exposing metrics for the application.
pub fn build_metric_layers(router: Router) -> anyhow::Result<Router> {
    let router = router
        .layer(middleware::from_fn(request_counter_middleware))
        .layer(middleware::from_fn(request_duration_middleware))
        .route("/metrics", get(metrics_endpoint));

    Ok(router)
}

/// Endpoint to return metrics for the application.
#[tracing::instrument()]
#[utoipa::path(
    get,
    path = "/metrics",
    responses((status = StatusCode::OK, description = "Application metrics"))
)]
async fn metrics_endpoint() -> Result<String, MetricsError> {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
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

/// Middleware to count number of requests.
async fn request_counter_middleware(request: Request<Body>, next: Next) -> Response {
    let uri = request.uri().clone();
    let method = request.method().clone();
    REQUEST_COUNTER
        .with_label_values(&[uri.path(), method.as_str()])
        .inc();
    REQUEST_ACTIVE_GAUGE.inc();

    // Run middleware chain
    let response = next.run(request).await;

    REQUEST_ACTIVE_GAUGE.dec();
    RESPONSE_COUNTER
        .with_label_values(&[uri.path(), method.as_str(), response.status().as_str()])
        .inc();

    response
}

/// Middleware to measure the duration of requests.
async fn request_duration_middleware(request: Request<Body>, next: Next) -> Response {
    let timer = REQUEST_DURATION
        .with_label_values(&[request.uri().path(), request.method().as_str()])
        .start_timer();
    let response = next.run(request).await;
    timer.stop_and_record();

    response
}
