use chrono::{DateTime, Utc};
use conservator::{Domain, Creatable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Supported provider types
pub const SUPPORTED_PROVIDERS: &[&str] = &["openai", "anthropic", "google"];

#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "provider_configs")]
pub struct ProviderConfig {
    #[domain(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_type: String,
    pub name: String,
    pub api_key_encrypted: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub is_active: bool,
    pub rate_limit: Option<i32>,
    pub monthly_quota: Option<f64>,
    pub used_quota: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating provider configurations
#[derive(Debug, Clone, Creatable)]
pub struct CreateProviderConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_type: String,
    pub name: String,
    pub api_key_encrypted: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub is_active: bool,
    pub rate_limit: Option<i32>,
    pub monthly_quota: Option<f64>,
    pub used_quota: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderConfigRequest {
    pub user_id: Option<Uuid>, // Optional, defaults to current user
    pub provider_type: String,  // "openai", "anthropic", "google", etc.
    pub name: String,
    pub api_key: String, // Plain text, will be encrypted
    pub base_url: Option<String>,
    pub priority: Option<i32>,
    pub rate_limit: Option<i32>,
    pub monthly_quota: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderConfigRequest {
    pub name: Option<String>,
    pub api_key: Option<String>, // If provided, will re-encrypt
    pub base_url: Option<String>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
    pub rate_limit: Option<i32>,
    pub monthly_quota: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ProviderConfigInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_type: String,
    pub name: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub is_active: bool,
    pub rate_limit: Option<i32>,
    pub monthly_quota: Option<f64>,
    pub used_quota: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProviderConfig> for ProviderConfigInfo {
    fn from(config: ProviderConfig) -> Self {
        Self {
            id: config.id,
            user_id: config.user_id,
            provider_type: config.provider_type,
            name: config.name,
            base_url: config.base_url,
            priority: config.priority,
            is_active: config.is_active,
            rate_limit: config.rate_limit,
            monthly_quota: config.monthly_quota,
            used_quota: config.used_quota,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

impl ProviderConfig {
    /// Validate if a provider type is supported
    pub fn is_valid_provider_type(provider_type: &str) -> bool {
        SUPPORTED_PROVIDERS.contains(&provider_type)
    }

    pub fn new(
        user_id: Uuid,
        provider_type: String,
        name: String,
        api_key_encrypted: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            provider_type,
            name,
            api_key_encrypted,
            base_url: None,
            priority: 0,
            is_active: true,
            rate_limit: None,
            monthly_quota: None,
            used_quota: 0.0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Encrypt API key using AES-256-GCM
    /// Format: base64(nonce || ciphertext || tag)
    pub fn encrypt_api_key(api_key: &str, secret: &str) -> String {
        use aes_gcm::{
            aead::{Aead, KeyInit, OsRng},
            Aes256Gcm, Nonce,
        };
        use base64::{Engine as _, engine::general_purpose};
        use sha2::{Digest, Sha256};

        // Derive a 256-bit key from the secret using SHA-256
        let key_bytes = Sha256::digest(secret.as_bytes());
        let cipher = Aes256Gcm::new(&key_bytes.into());

        // Generate a random 96-bit nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the API key
        let ciphertext = cipher
            .encrypt(nonce, api_key.as_bytes())
            .expect("encryption failed");

        // Combine nonce + ciphertext and encode to base64
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        general_purpose::STANDARD.encode(&combined)
    }

    /// Decrypt API key using AES-256-GCM
    pub fn decrypt_api_key(encrypted: &str, secret: &str) -> Result<String, String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use base64::{Engine as _, engine::general_purpose};
        use sha2::{Digest, Sha256};

        // Derive the same 256-bit key from the secret
        let key_bytes = Sha256::digest(secret.as_bytes());
        let cipher = Aes256Gcm::new(&key_bytes.into());

        // Decode from base64
        let combined = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| format!("Base64 decode error: {}", e))?;

        // Split nonce (12 bytes) and ciphertext
        if combined.len() < 12 {
            return Err("Invalid encrypted data: too short".to_string());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode error: {}", e))
    }
}

#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub user_id: Uuid,
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub by_model: Vec<ModelUsageStats>,
    pub by_provider: Vec<ProviderUsageStats>,
}

#[derive(Debug, Serialize)]
pub struct ModelUsageStats {
    pub model: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct ProviderUsageStats {
    pub provider: String,
    pub requests: i64,
    pub total_tokens: i64,
    pub cost: f64,
}
