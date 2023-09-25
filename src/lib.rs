pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod error;
mod routes;
mod state;
pub mod telemetry;

use axum::{Router, Server};
use configuration::Settings;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::{net::TcpListener, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    request_id::MakeRequestUuid,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::Level;

#[derive(Debug)]
pub struct App {
    listener: TcpListener,
    router: Router,
}

impl App {
    pub async fn build(config: Settings) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(config.application().address())?;
        let db_pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(2))
            .connect_lazy_with(config.database().with_db());

        let email_client = config
            .email_client()
            .try_into()
            .expect("Failed to create email client");

        let app_state = AppState::create(config, db_pool, email_client).await;
        let router = Self::build_router(&app_state);

        Ok(Self { listener, router })
    }

    /// Run the server until it is stopped.
    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        tracing::info!("Server running at {}", self.listener.local_addr()?);

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
    fn build_router(app_state: &AppState) -> Router {
        routes::build_router(app_state).layer(
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
}
