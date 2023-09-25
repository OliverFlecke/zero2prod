use crate::{configuration::Settings, email_client::EmailClient};
use axum::extract::FromRef;
use derive_getters::Getters;
use duplicate::duplicate_item;
use sqlx::PgPool;
use std::sync::Arc;
use tera::Tera;

#[derive(Debug, Clone, Getters)]
pub struct AppState {
    db_pool: Arc<PgPool>,
    email_client: Arc<EmailClient>,
    application_base_url: Arc<ApplicationBaseUrl>,
    view_templates: Arc<Tera>,
}

impl AppState {
    pub async fn create(config: Settings, db_pool: PgPool, email_client: EmailClient) -> Self {
        let view_templates = Tera::new("src/views/**/*").expect("Templates could not be loaded");

        Self {
            db_pool: Arc::new(db_pool),
            email_client: Arc::new(email_client),
            application_base_url: Arc::new(ApplicationBaseUrl(config.application.base_url.clone())),
            view_templates: Arc::new(view_templates),
        }
    }
}

#[duplicate_item(
    service_type            field;
    [ PgPool ]              [ db_pool ];
    [ EmailClient ]         [ email_client ];
    [ ApplicationBaseUrl ]  [ application_base_url ];
    [ Tera ]                [ view_templates ];
)]
impl FromRef<AppState> for Arc<service_type> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.field.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationBaseUrl(pub String);
