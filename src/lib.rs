pub mod configuration;
mod routes;
mod state;
pub mod telemetry;

use axum::{Router, Server};
use sqlx::PgPool;
use state::AppState;
use std::net::TcpListener;

#[derive(Debug)]
pub struct App;

impl App {
    /// Serve this app on the given [`TcpListener`].
    pub async fn serve(host: TcpListener, db_pool: PgPool) -> anyhow::Result<()> {
        tracing::info!("Server running at {}", host.local_addr()?);
        let app_state = AppState::create(db_pool).await;
        let router = Self::build_router(&app_state);

        Server::from_tcp(host)?
            .serve(router.into_make_service())
            .await?;
        Ok(())
    }

    /// Builder the router for the application.
    fn build_router(app_state: &AppState) -> Router {
        use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
        use tracing::Level;

        Router::new()
            .nest("/health", routes::health::create_router())
            .nest(
                "/subscriptions",
                routes::subscribe::create_router().with_state(app_state.clone()),
            )
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
    }
}
