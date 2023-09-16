use crate::email_client::EmailClient;
use axum::extract::FromRef;
use derive_getters::Getters;
use duplicate::duplicate_item;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone, Getters)]
pub struct AppState {
    db_pool: Arc<PgPool>,
    email_client: Arc<EmailClient>,
}

impl AppState {
    pub async fn create(db_pool: PgPool, email_client: EmailClient) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
            email_client: Arc::new(email_client),
        }
    }
}

#[duplicate_item(
    service_type    field;
    [ PgPool ]      [ db_pool ];
    [ EmailClient ] [ email_client ];
)]
impl FromRef<AppState> for Arc<service_type> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.field.clone()
    }
}
