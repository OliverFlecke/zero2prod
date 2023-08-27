mod routes;

use axum::{Router, Server};
use std::net::TcpListener;

#[derive(Debug)]
pub struct App {
    router: Router,
}

impl App {
    /// Create a new instance of the app with its router and state,
    /// which can then be served with [`serve`].
    pub fn create() -> Self {
        let router = Self::build_router();

        Self { router }
    }

    /// Serve this app on the given [`TcpListener`].
    pub async fn serve(self, host: TcpListener) -> anyhow::Result<()> {
        Self::setup_tracing()?;
        tracing::info!("Server running at {}", host.local_addr()?);

        Server::from_tcp(host)?
            .serve(self.router.into_make_service())
            .await?;
        Ok(())
    }

    /// Builder the router for the application.
    fn build_router() -> Router {
        Router::new().nest("/health", routes::health::create_router())
        // .fallback(not_found)
    }

    fn setup_tracing() -> anyhow::Result<()> {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("zero2prod=debug".parse()?)
                    .add_directive("hyper=info".parse()?)
                    .add_directive("tower_http=info".parse()?),
            )
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
        Ok(())
    }
}
