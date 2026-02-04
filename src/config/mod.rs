use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: i64, // seconds
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start with defaults
            .add_source(File::with_name("config/default").required(false))
            // Add environment-specific file
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Add local configuration file (not tracked by git)
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from environment variables (with prefix MANTI)
            .add_source(Environment::with_prefix("MANTI").separator("_"))
            .build()?;

        s.try_deserialize()
    }
}