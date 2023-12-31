pub mod authorization;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod error;
pub(crate) mod idempotency;
pub mod issue_delivery_worker;
mod metrics;
pub(crate) mod require_login;
mod routes;
pub(crate) mod service;
mod state;
pub mod telemetry;

use crate::require_login::AuthorizedUser;
use anyhow::Context;
use axum::{
    error_handling::HandleErrorLayer, middleware::from_extractor_with_state, BoxError, Router,
};
use configuration::Settings;
use http::StatusCode;
use sqlx::{postgres::PgPoolOptions, PgPool};
use state::AppState;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{
    request_id::MakeRequestUuid,
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tower_sessions::{
    fred::{
        prelude::{ClientLike, RedisClient},
        types::RedisConfig,
    },
    RedisStore, SessionManagerLayer,
};
use tracing::Level;

/// Application container for the service itself.
#[derive(Debug)]
pub struct App {
    listener: TcpListener,
    router: Router,
}

impl App {
    pub async fn build(config: Settings) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(config.application().address()).await?;
        let db_pool = get_connection_pool(&config);

        let email_client = config
            .email_client()
            .try_into()
            .expect("Failed to create email client");
        let redis_client = create_and_connect_redis_client(&config).await?;
        let app_state = AppState::create(&config, db_pool, email_client, redis_client).await;
        let router = Self::build_router(&config, &app_state).await?;

        Ok(Self { listener, router })
    }

    /// Run the server until it is stopped.
    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        tracing::info!(
            "Server running at {}. Version: {}",
            self.listener.local_addr()?,
            env!("CARGO_PKG_VERSION")
        );

        axum::serve(self.listener, self.router.into_make_service()).await?;
        Ok(())
    }

    /// Get the port which the server is being run on.
    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    /// Builder the router for the application.
    async fn build_router(config: &Settings, app_state: &AppState) -> anyhow::Result<Router> {
        let redis_client = create_and_connect_redis_client(config).await?;

        use routes::*;
        let router = Router::new()
            .nest("/", home::create_router().with_state(app_state.clone()))
            .nest(
                "/login",
                login::create_router().with_state(app_state.clone()),
            )
            .nest(
                "/admin",
                admin::create_router()
                    // Enforce authorized user on all admin endpoints.
                    .route_layer(from_extractor_with_state::<AuthorizedUser, AppState>(
                        app_state.clone(),
                    ))
                    .with_state(app_state.clone()),
            )
            .nest(
                "/subscriptions",
                subscriptions::create_router().with_state(app_state.clone()),
            )
            .add_session_layer(redis_client)
            // Routes after this layer does not have access to the user sessions.
            .nest_service("/assets", ServeDir::new("assets"))
            .nest("/docs", docs::create_router())
            .nest("/", health::create_router().with_state(app_state.clone()));

        Ok(router
            .add_telemetry_layer()
            .add_metrics_layer()
            .add_error_handling_layer())
    }
}

pub fn get_connection_pool(configuration: &Settings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database().with_db())
}

/// Create a client for Redis and connect it.
async fn create_and_connect_redis_client(config: &Settings) -> anyhow::Result<RedisClient> {
    use secrecy::ExposeSecret;
    let client = RedisClient::new(
        RedisConfig::from_url(config.redis().url().expose_secret())?,
        None,
        None,
        None,
    );

    #[allow(clippy::let_underscore_future)]
    let _ = client.connect();
    client
        .wait_for_connect()
        .await
        .context("Unable to connect Redis client")?;

    Ok(client)
}

/// Utility trait to help setup different layers on the router.
trait AddRouterLayer {
    fn add_error_handling_layer(self) -> Self;

    fn add_telemetry_layer(self) -> Self;

    fn add_metrics_layer(self) -> Self;

    fn add_session_layer(self, redis_client: RedisClient) -> Self;
}

impl AddRouterLayer for Router {
    fn add_error_handling_layer(self) -> Self {
        self.layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    tracing::error!("Request timed out: {e:?}");
                    http::StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(10))),
        )
    }

    fn add_telemetry_layer(self) -> Self {
        self.layer(
            ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid)
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            DefaultMakeSpan::new()
                                .level(Level::INFO)
                                .include_headers(true),
                        )
                        .on_request(DefaultOnRequest::new().level(Level::INFO))
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .include_headers(true),
                        ),
                )
                .propagate_x_request_id(),
        )
    }

    fn add_metrics_layer(self) -> Self {
        crate::metrics::build_metric_layers(self)
            .expect("metrics layer should always be possible to setup")
    }

    fn add_session_layer(self, redis_client: RedisClient) -> Self {
        let store = RedisStore::new(redis_client);

        self.layer(
            ServiceBuilder::new()
                // Note: Why is this error handling layer needed? The types won't match otherwise for the session layer.
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::BAD_REQUEST
                }))
                .layer(SessionManagerLayer::new(store).with_secure(true)),
        )
    }
}
