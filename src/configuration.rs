use config::{Config, File};
use derive_getters::Getters;
use secrecy::{ExposeSecret, Secret};

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
        .build()?
        .try_deserialize()
}

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
#[derive(Debug, serde::Deserialize, Getters)]
pub struct Settings {
    pub database: DatabaseSettings,
    application: ApplicationSettings,
}

/// General application settings.
#[derive(Debug, serde::Deserialize, Getters)]
pub struct ApplicationSettings {
    port: u16,
    host: String,
}

impl ApplicationSettings {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Settings for connecting to the database.
#[derive(Debug, serde::Deserialize, Getters)]
pub struct DatabaseSettings {
    username: String,
    password: Secret<String>,
    port: u16,
    host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    /// Get the connection string to the database.
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        ))
    }

    /// Get the connection string to the postgres instance, but without a
    /// specific database.
    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}
