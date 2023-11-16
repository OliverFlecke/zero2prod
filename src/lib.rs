pub mod authorization;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod error;
pub(crate) mod idempotency;
pub mod issue_delivery_worker;
pub(crate) mod require_login;
mod routes;
pub(crate) mod service;
mod state;
pub mod telemetry;

use crate::require_login::AuthorizedUser;
use async_redis_session::RedisSessionStore;
use axum::{
    error_handling::HandleErrorLayer, middleware::from_extractor_with_state, BoxError, Router,
    Server,
};
use axum_sessions::SessionLayer;
use configuration::Settings;
use sqlx::{postgres::PgPoolOptions, PgPool};
use state::AppState;
use std::{net::TcpListener, time::Duration};
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{
    request_id::MakeRequestUuid,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
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
        let listener = TcpListener::bind(config.application().address())?;
        let db_pool = get_connection_pool(&config);

        let email_client = config
            .email_client()
            .try_into()
            .expect("Failed to create email client");
        let redis_client = redis::Client::open(
            secrecy::ExposeSecret::expose_secret(&config.redis().url()).as_str(),
        )?;

        let app_state = AppState::create(&config, db_pool, email_client, redis_client).await;
        let router = Self::build_router(&config, &app_state)?;

        Ok(Self { listener, router })
    }

    /// Run the server until it is stopped.
    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        tracing::info!(
            "Server running at {}. Version: {}",
            self.listener.local_addr()?,
            env!("CARGO_PKG_VERSION")
        );

        Server::from_tcp(self.listener)?
            .serve(self.router.into_make_service())
            .await?;
        Ok(())
    }

    /// Get the port which the server is being run on.
    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    /// Builder the router for the application.
    fn build_router(config: &Settings, app_state: &AppState) -> anyhow::Result<Router> {
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
            .layer(Self::build_session_layer(config)?)
            // Routes after this layer does not have access to the user sessions.
            .nest("/", health::create_router().with_state(app_state.clone()))
            .nest("/", docs::create_router());

        Ok(router.layer(
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
                .propagate_x_request_id()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    tracing::error!("Request timed out: {e:?}");
                    http::StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(10))),
        ))
    }

    /// Create a session layer with a redis backend store.
    fn build_session_layer(config: &Settings) -> anyhow::Result<SessionLayer<RedisSessionStore>> {
        use secrecy::ExposeSecret;

        let store = RedisSessionStore::new(config.redis().url().expose_secret().as_str())?;
        let secret = config
            .application()
            .hmac_secret()
            .expose_secret()
            .as_bytes();

        Ok(SessionLayer::new(store, secret))
    }
}

pub fn get_connection_pool(configuration: &Settings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database().with_db())
}
