use opentelemetry::{runtime, KeyValue};
use opentelemetry_sdk::{
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing::{subscriber::set_global_default, Level, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    filter, fmt::MakeWriter, layer::SubscriberExt, registry::LookupSpan, Registry,
};

/// Create a new subscriber to add telemetry to the application.
pub fn get_subscriber<Sink>(
    name: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync + for<'span> LookupSpan<'span>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let filter = filter::Targets::new()
        .with_target("zero2prod", Level::DEBUG)
        .with_target("tower_http::trace", Level::INFO)
        .with_target("hyper", Level::INFO)
        .with_default(Level::WARN);

    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Init a subscriber and set it as the global tracing subscription.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn setup_optl(
    subscriber: impl Subscriber + Send + Sync + for<'span> LookupSpan<'span>,
) -> impl Subscriber + Send + Sync + for<'span> LookupSpan<'span> {
    subscriber.with(OpenTelemetryLayer::new(init_tracer()))
}

// Construct Tracer for OpenTelemetryLayer
fn init_tracer() -> Tracer {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // Customize sampling strategy
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                // If export trace to AWS X-Ray, you can use XrayIdGenerator
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource()),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(runtime::Tokio)
        .unwrap()
}

fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "develop"),
        ],
        SCHEMA_URL,
    )
}
