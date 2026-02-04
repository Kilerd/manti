use crate::models::{api_key::ApiKey, usage::Usage, user::User};
use chrono::{DateTime, Utc};
use conservator::{Executor, PooledConnection};
use std::sync::Arc;
use uuid::Uuid;

/// Database service for managing all database operations
pub struct DatabaseService {
    pool: Arc<PooledConnection>,
}

impl DatabaseService {
    /// Create a new database service
    pub fn new(database_url: &str) -> crate::Result<Self> {
        let pool = PooledConnection::from_url(database_url)
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Read and execute migration files
        let migrations = vec![
            include_str!("../../migrations/001_initial.sql"),
            include_str!("../../migrations/002_auth.sql"),
        ];

        for (i, migration) in migrations.iter().enumerate() {
            conn.execute(migration, &[]).await
                .map_err(|e| crate::MantiError::Database(e))?;
            tracing::info!("Applied migration {}", i + 1);
        }

        Ok(())
    }

    // User operations

    /// Create a new user
    pub async fn create_user(&self, user: &User) -> crate::Result<User> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            INSERT INTO users (id, email, username, password_hash, is_active, is_admin, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
        "#;

        let row = conn
            .query_one(
                query,
                &[
                    &user.id,
                    &user.email,
                    &user.username,
                    &user.password_hash,
                    &user.is_active,
                    &user.is_admin,
                    &user.created_at,
                    &user.updated_at,
                ],
            )
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row_to_user(&row))
    }

    /// Find a user by email
    pub async fn find_user_by_email(&self, email: &str) -> crate::Result<Option<User>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "SELECT * FROM users WHERE email = $1";

        let row = conn
            .query_opt(query, &[&email])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row.map(|r| row_to_user(&r)))
    }

    /// Find a user by ID
    pub async fn find_user_by_id(&self, id: Uuid) -> crate::Result<Option<User>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "SELECT * FROM users WHERE id = $1";

        let row = conn
            .query_opt(query, &[&id])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row.map(|r| row_to_user(&r)))
    }

    /// Update user last login
    pub async fn update_user_last_login(&self, user_id: Uuid) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "UPDATE users SET last_login = $1, updated_at = $2 WHERE id = $3";
        let now = Utc::now();

        conn.execute(query, &[&now, &now, &user_id])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // API Key operations

    /// Create a new API key
    pub async fn create_api_key(&self, api_key: &ApiKey) -> crate::Result<ApiKey> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            INSERT INTO api_keys (
                id, user_id, name, key_hash, prefix, is_active,
                expires_at, created_at, updated_at, rate_limit_rpm, allowed_models
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
        "#;

        let allowed_models_json = api_key.allowed_models.as_ref()
            .map(|models| serde_json::to_value(models).unwrap());

        let row = conn
            .query_one(
                query,
                &[
                    &api_key.id,
                    &api_key.user_id,
                    &api_key.name,
                    &api_key.key_hash,
                    &api_key.prefix,
                    &api_key.is_active,
                    &api_key.expires_at,
                    &api_key.created_at,
                    &api_key.updated_at,
                    &api_key.rate_limit_rpm,
                    &allowed_models_json,
                ],
            )
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row_to_api_key(&row))
    }

    /// Find an API key by its prefix and verify with hash
    pub async fn find_and_verify_api_key(&self, key: &str) -> crate::Result<Option<ApiKey>> {
        // Extract prefix for faster lookup
        let prefix = key.chars().take(8).collect::<String>();

        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "SELECT * FROM api_keys WHERE prefix = $1 AND is_active = true";

        let row = conn
            .query_opt(query, &[&prefix])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        if let Some(row) = row {
            let api_key = row_to_api_key(&row);

            // Verify the full key
            if api_key.verify(key) && api_key.is_valid() {
                Ok(Some(api_key))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Find an API key by prefix (for identifying which key was used)
    pub async fn find_api_key_by_prefix(&self, prefix: &str) -> crate::Result<Option<ApiKey>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "SELECT * FROM api_keys WHERE prefix = $1 AND is_active = true";

        let row = conn
            .query_opt(query, &[&prefix])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row.map(|r| row_to_api_key(&r)))
    }

    /// List all API keys for a user
    pub async fn list_user_api_keys(&self, user_id: Uuid) -> crate::Result<Vec<ApiKey>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC";

        let rows = conn
            .query(query, &[&user_id])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(rows.iter().map(|r| row_to_api_key(r)).collect())
    }

    /// Update API key last used
    pub async fn update_api_key_last_used(&self, api_key_id: Uuid) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "UPDATE api_keys SET last_used = $1, updated_at = $2 WHERE id = $3";
        let now = Utc::now();

        conn.execute(query, &[&now, &now, &api_key_id])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, api_key_id: Uuid, user_id: Uuid) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = "UPDATE api_keys SET is_active = false, updated_at = $1 WHERE id = $2 AND user_id = $3";

        conn.execute(query, &[&Utc::now(), &api_key_id, &user_id])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // Usage operations

    /// Record API usage
    pub async fn record_usage(&self, usage: &Usage) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            INSERT INTO usage (
                id, user_id, api_key_id, model, provider,
                prompt_tokens, completion_tokens, total_tokens,
                cost, request_id, created_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#;

        conn.execute(
            query,
            &[
                &usage.id,
                &usage.user_id,
                &usage.api_key_id,
                &usage.model,
                &usage.provider,
                &usage.prompt_tokens,
                &usage.completion_tokens,
                &usage.total_tokens,
                &usage.cost,
                &usage.request_id,
                &usage.created_at,
                &usage.metadata,
            ],
        )
        .await
        .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    /// Get usage summary for a user in a time period
    pub async fn get_usage_summary(
        &self,
        user_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> crate::Result<Vec<Usage>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            SELECT * FROM usage
            WHERE user_id = $1 AND created_at >= $2 AND created_at <= $3
            ORDER BY created_at DESC
        "#;

        let rows = conn
            .query(query, &[&user_id, &start, &end])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(rows.iter().map(|r| row_to_usage(r)).collect())
    }
}

// Helper functions to convert database rows to models

fn row_to_user(row: &conservator::Row) -> User {
    User {
        id: row.get("id"),
        email: row.get("email"),
        username: row.get("username"),
        password_hash: row.get("password_hash"),
        is_active: row.get("is_active"),
        is_admin: row.get("is_admin"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_login: row.get("last_login"),
    }
}

fn row_to_api_key(row: &conservator::Row) -> ApiKey {
    let allowed_models: Option<serde_json::Value> = row.get("allowed_models");
    let allowed_models = allowed_models.and_then(|v| {
        serde_json::from_value::<Vec<String>>(v).ok()
    });

    ApiKey {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        key_hash: row.get("key_hash"),
        prefix: row.get("prefix"),
        is_active: row.get("is_active"),
        last_used: row.get("last_used"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        rate_limit_rpm: row.get("rate_limit_rpm"),
        allowed_models,
    }
}

fn row_to_usage(row: &conservator::Row) -> Usage {
    Usage {
        id: row.get("id"),
        user_id: row.get("user_id"),
        api_key_id: row.get("api_key_id"),
        model: row.get("model"),
        provider: row.get("provider"),
        prompt_tokens: row.get("prompt_tokens"),
        completion_tokens: row.get("completion_tokens"),
        total_tokens: row.get("total_tokens"),
        cost: row.get("cost"),
        request_id: row.get("request_id"),
        created_at: row.get("created_at"),
        metadata: row.get("metadata"),
    }
}