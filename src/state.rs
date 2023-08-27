use axum::extract::FromRef;
use derive_getters::Getters;
use duplicate::duplicate_item;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone, Getters)]
pub struct AppState {
    db_pool: Arc<PgPool>,
}

impl AppState {
    pub async fn create(db_pool: PgPool) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
        }
    }
}

#[duplicate_item(
    service_type    field;
    [ PgPool ]      [ db_pool ];
)]
impl FromRef<AppState> for Arc<service_type> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.field.clone()
    }
}
