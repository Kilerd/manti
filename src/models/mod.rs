pub mod chat;
pub mod streaming;
pub mod response;
pub mod user;
pub mod api_key;
pub mod usage;
pub mod provider_config;

use conservator::{PooledConnection, Error};

pub struct Database {
    pub pool: PooledConnection,
}

impl Database {
    pub async fn new(database_url: &str, _max_connections: u32) -> Result<Self, Error> {
        let pool = PooledConnection::from_url(database_url)?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        // TODO: Implement migrations
        // conservator doesn't have built-in migration support like sqlx
        // You'll need to handle migrations separately
        Ok(())
    }
}