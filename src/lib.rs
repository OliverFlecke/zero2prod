pub mod configuration;
mod routes;
mod state;

use axum::{Router, Server};
use sqlx::PgPool;
use state::AppState;
use std::net::TcpListener;

#[derive(Debug)]
pub struct App;

impl App {
    /// Serve this app on the given [`TcpListener`].
    pub async fn serve(host: TcpListener, db_pool: PgPool) -> anyhow::Result<()> {
        // Self::setup_tracing()?;
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
        Router::new()
            .nest("/health", routes::health::create_router())
            .nest(
                "/subscriptions",
                routes::subscribe::create_router().with_state(app_state.clone()),
            )
    }

    // fn setup_tracing() -> anyhow::Result<()> {
    //     use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    //     tracing_subscriber::registry()
    //         .with(
    //             tracing_subscriber::EnvFilter::from_default_env()
    //                 .add_directive("zero2prod=debug".parse()?)
    //                 .add_directive("hyper=info".parse()?)
    //                 .add_directive("tower_http=info".parse()?),
    //         )
    //         .with(tracing_subscriber::fmt::layer().compact())
    //         .init();
    //     Ok(())
    // }
}
