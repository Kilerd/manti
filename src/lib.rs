pub mod config;
pub mod models;
pub mod providers;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MantiError {
    #[error("Database error: {0}")]
    Database(#[from] conservator::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Billing error: {0}")]
    Billing(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, MantiError>;