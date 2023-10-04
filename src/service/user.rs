use crate::state::AppState;
use anyhow::Context;
use axum::extract::FromRef;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Service around user related services.
pub struct UserService {
    db_pool: Arc<PgPool>,
}

impl UserService {
    /// Get a user's username from their id.
    #[tracing::instrument(name = "Get username", skip(self))]
    pub async fn get_username(&self, user_id: &Uuid) -> Result<String, anyhow::Error> {
        let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
            .fetch_one(self.db_pool.as_ref())
            .await
            .context("Failed to perform a query to retreive a username")?;

        Ok(row.username)
    }
}

impl FromRef<AppState> for UserService {
    fn from_ref(state: &AppState) -> Self {
        UserService {
            db_pool: state.db_pool().clone(),
        }
    }
}
