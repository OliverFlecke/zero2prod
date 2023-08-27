use tracing::{subscriber::set_global_default, Level, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{filter, fmt::MakeWriter, layer::SubscriberExt, Registry};

/// Create a new subscriber to add telemetry to the application.
pub fn get_subscriber<Sink>(name: String, sink: Sink) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let filter = filter::Targets::new()
        .with_target("zero2prod", Level::DEBUG)
        .with_target("tower_http::trace", Level::INFO)
        .with_target("hyper", Level::INFO)
        .with_default(Level::WARN);

    let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);

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
