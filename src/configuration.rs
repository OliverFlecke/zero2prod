use config::{Config, File};
use derive_getters::Getters;
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};
use std::time::Duration;

use crate::domain::SubscriberEmail;

/// Retrive the configuration for the application.
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.yaml", environment.as_str());

    Config::builder()
        .add_source(File::from(configuration_directory.join("base.yaml")))
        .add_source(File::from(
            configuration_directory.join(environment_filename),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize()
}

/// Environmnet to run the application in. Used to determine which configuration
/// to use.
#[derive(Debug)]
enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. \
                Use either `local` or `production`.",
            )),
        }
    }
}

/// Settings
#[derive(Debug, Clone, serde::Deserialize, Getters)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

/// General application settings.
#[derive(Debug, Clone, serde::Deserialize, Getters)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    hmac_secret: Secret<String>,
}

impl ApplicationSettings {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Settings for connecting to the database.
#[derive(Debug, Clone, serde::Deserialize, Getters)]
pub struct DatabaseSettings {
    username: String,
    password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
    host: String,
    pub name: String,
    require_ssl: bool,
}

impl DatabaseSettings {
    /// Get the connection string to the database.
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(self.name())
    }

    /// Get the connection string to the postgres instance, but without a
    /// specific database.
    pub fn without_db(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(self.host())
            .username(self.username())
            .password(self.password().expose_secret())
            .port(self.port)
            .ssl_mode(if self.require_ssl {
                PgSslMode::Require
            } else {
                PgSslMode::Prefer
            })
            .log_statements(tracing_log::log::LevelFilter::Trace)
    }
}

/// Settings for the email client.
#[derive(Debug, Clone, serde::Deserialize, Getters)]
pub struct EmailClientSettings {
    #[getter(skip)]
    pub base_url: String,
    #[getter(skip)]
    sender: String,
    authorization_token: Secret<String>,
    #[getter(skip)]
    timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender.clone())
    }

    pub fn base_url(&self) -> Result<reqwest::Url, url::ParseError> {
        reqwest::Url::parse(&self.base_url)
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }
}
