use chrono::{DateTime, Utc};
use conservator::{Domain, Creatable};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Usage record for tracking API usage and billing
#[derive(Debug, Clone, Serialize, Deserialize, Domain, Schematic)]
#[domain(table = "usage")]
pub struct Usage {
    #[domain(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,  // in USD
    pub request_id: String,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<JsonValue>,
}

/// DTO for creating usage records
#[derive(Debug, Clone, Creatable)]
pub struct CreateUsage {
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
    pub request_id: String,
    pub metadata: Option<JsonValue>,
}

impl CreateUsage {
    pub fn new(
        user_id: Uuid,
        api_key_id: Option<Uuid>,
        model: String,
        provider: String,
        prompt_tokens: i64,
        completion_tokens: i64,
        cost: f64,
    ) -> Self {
        Self {
            user_id,
            api_key_id,
            model,
            provider,
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cost,
            request_id: Uuid::new_v4().to_string(),
            metadata: None,
        }
    }
}

impl Usage {
    pub fn new(
        user_id: Uuid,
        api_key_id: Option<Uuid>,
        model: String,
        provider: String,
        prompt_tokens: i64,
        completion_tokens: i64,
        cost: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            api_key_id,
            model,
            provider,
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cost,
            request_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            metadata: None,
        }
    }
}

/// Usage summary for a time period
#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub by_model: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub provider: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

/// Billing record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Billing {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: f64,
    pub status: BillingStatus,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillingStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
}