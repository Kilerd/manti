use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API Key for authenticating API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub prefix: String,  // First 8 chars of key for identification
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rate_limit_rpm: Option<i32>,  // Requests per minute
    pub allowed_models: Option<Vec<String>>,  // Optional model restrictions
}

impl ApiKey {
    /// Generate a new API key
    pub fn generate(user_id: Uuid, name: String) -> crate::Result<(Self, String)> {
        let raw_key = generate_api_key();
        let key_hash = hash_api_key(&raw_key)?;
        let prefix = raw_key.chars().take(8).collect::<String>();

        let api_key = Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            key_hash,
            prefix,
            is_active: true,
            last_used: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            rate_limit_rpm: Some(60),  // Default 60 requests per minute
            allowed_models: None,
        };

        Ok((api_key, raw_key))
    }

    /// Verify an API key
    pub fn verify(&self, key: &str) -> bool {
        verify_api_key(key, &self.key_hash).unwrap_or(false)
    }

    /// Check if the key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }

    /// Update last used timestamp
    pub fn update_last_used(&mut self) {
        self.last_used = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if a model is allowed for this key
    pub fn is_model_allowed(&self, model: &str) -> bool {
        match &self.allowed_models {
            Some(models) => models.contains(&model.to_string()),
            None => true,  // No restrictions means all models are allowed
        }
    }
}

/// Generate a random API key
fn generate_api_key() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const KEY_LEN: usize = 48;

    let mut rng = rand::thread_rng();

    let key: String = (0..KEY_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("sk-manti-{}", key)
}

/// Hash an API key
fn hash_api_key(key: &str) -> crate::Result<String> {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();

    Ok(format!("{:x}", result))
}

/// Verify an API key against a hash
fn verify_api_key(key: &str, hash: &str) -> crate::Result<bool> {
    let computed_hash = hash_api_key(key)?;
    Ok(computed_hash == hash)
}

/// API key creation request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<i64>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
}

/// API key response (for creation only, includes the raw key once)
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String,  // Only returned on creation
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// API key info (without sensitive data)
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
}