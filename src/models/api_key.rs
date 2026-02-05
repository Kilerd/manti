use chrono::{DateTime, Utc};
use conservator::{Domain, Creatable};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// API Key for authenticating API requests
#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "api_keys")]
pub struct ApiKey {
    #[domain(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key: String,
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<JsonValue>,
}

/// DTO for creating new API keys
#[derive(Debug, Clone, Creatable)]
pub struct CreateApiKey {
    pub user_id: Uuid,
    pub name: String,
    pub key: String,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<JsonValue>,
}

impl CreateApiKey {
    /// Generate a new API key DTO
    pub fn generate(
        user_id: Uuid,
        name: String,
        expires_at: Option<DateTime<Utc>>,
        rate_limit_rpm: Option<i32>,
        allowed_models: Option<Vec<String>>,
    ) -> Self {
        let key = generate_api_key();
        let allowed_models_json = allowed_models.map(|models| serde_json::to_value(models).unwrap());

        Self {
            user_id,
            name,
            key,
            is_active: true,
            expires_at,
            rate_limit_rpm: rate_limit_rpm.or(Some(60)),
            allowed_models: allowed_models_json,
        }
    }
}

impl ApiKey {
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

    /// Check if a model is allowed for this key
    pub fn is_model_allowed(&self, model: &str) -> bool {
        match &self.allowed_models {
            Some(json_models) => {
                if let Ok(models) = serde_json::from_value::<Vec<String>>(json_models.clone()) {
                    models.contains(&model.to_string())
                } else {
                    true
                }
            }
            None => true,
        }
    }

    /// Get allowed models as Vec<String>
    pub fn get_allowed_models(&self) -> Option<Vec<String>> {
        self.allowed_models.as_ref().and_then(|json| {
            serde_json::from_value::<Vec<String>>(json.clone()).ok()
        })
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

/// API key creation request
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<i64>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
}

/// API key info for listing
#[derive(Debug, Clone, Serialize, Schematic)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub rate_limit_rpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
}

impl From<ApiKey> for ApiKeyInfo {
    fn from(api_key: ApiKey) -> Self {
        let allowed_models = api_key.get_allowed_models();
        Self {
            id: api_key.id,
            name: api_key.name,
            key: api_key.key,
            is_active: api_key.is_active,
            last_used: api_key.last_used,
            expires_at: api_key.expires_at,
            created_at: api_key.created_at,
            rate_limit_rpm: api_key.rate_limit_rpm,
            allowed_models,
        }
    }
}
