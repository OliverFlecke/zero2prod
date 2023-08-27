use config::{Config, File, FileFormat};
use derive_getters::Getters;

/// Retrive the configuration for the application.
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    Config::builder()
        .add_source(File::new("configuration.yaml", FileFormat::Yaml))
        .build()?
        .try_deserialize()
}

#[derive(Debug, serde::Deserialize, Getters)]
pub struct Settings {
    pub database: DatabaseSettings,
    application_port: u16,
}

#[derive(Debug, serde::Deserialize, Getters)]
pub struct DatabaseSettings {
    username: String,
    password: String,
    port: u16,
    host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    /// Get the connection string to the database.
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    /// Get the connection string to the postgres instance, but without a
    /// specific database.
    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}
