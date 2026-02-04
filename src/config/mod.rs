use config::{Config, ConfigError, Environment, File};
use gotcha::ConfigWrapper;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: i64, // seconds
}

impl Settings {
    pub fn new() -> Result<ConfigWrapper<Self>, ConfigError> {
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
