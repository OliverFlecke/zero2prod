use crate::{configuration::Settings, email_client::EmailClient};
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key as CookieKey;
use derive_getters::Getters;
use duplicate::duplicate_item;
use secrecy::Secret;
use sqlx::PgPool;
use std::sync::Arc;
use tower_sessions::fred::prelude::RedisClient;

pub mod session;

#[derive(Clone, Getters)]
pub struct AppState {
    db_pool: Arc<PgPool>,
    redis_client: Arc<RedisClient>,
    email_client: Arc<EmailClient>,
    application_base_url: Arc<ApplicationBaseUrl>,
    hmac_secret: Arc<HmacSecret>,
    cookie_key: CookieKey,
}

impl AppState {
    /// Create a new container for all of the app state.
    pub async fn create(
        config: &Settings,
        db_pool: PgPool,
        email_client: EmailClient,
        redis_client: RedisClient,
    ) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
            redis_client: Arc::new(redis_client),
            email_client: Arc::new(email_client),
            application_base_url: Arc::new(ApplicationBaseUrl(
                config.application().base_url().clone(),
            )),
            hmac_secret: Arc::new(HmacSecret(config.application().hmac_secret().clone())),
            cookie_key: CookieKey::generate(),
        }
    }
}

#[duplicate_item(
    service_type            field;
    [ PgPool ]              [ db_pool ];
    [ EmailClient ]         [ email_client ];
    [ ApplicationBaseUrl ]  [ application_base_url ];
    [ HmacSecret ]          [ hmac_secret ];
    [ RedisClient ]         [ redis_client ];
)]
impl FromRef<AppState> for Arc<service_type> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.field.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationBaseUrl(pub String);

pub struct HmacSecret(pub Secret<String>);

/// Allows for extraction of the signing key for cookies.
impl FromRef<AppState> for CookieKey {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
