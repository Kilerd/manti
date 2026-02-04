pub mod config;
pub mod models;
pub mod providers;
pub mod db;
pub mod auth;
pub mod api;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MantiError {
    #[error("Database error: {0}")]
    Database(#[from] conservator::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] conservator::MigrateError),

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

impl MantiError {
    pub fn to_status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            MantiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Migration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Auth(_) => StatusCode::UNAUTHORIZED,
            MantiError::Provider(_) => StatusCode::BAD_GATEWAY,
            MantiError::Billing(_) => StatusCode::PAYMENT_REQUIRED,
            MantiError::Validation(_) => StatusCode::BAD_REQUEST,
            MantiError::NotFound(_) => StatusCode::NOT_FOUND,
            MantiError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            MantiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}