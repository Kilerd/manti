use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain};
use gotcha::Schematic;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Billing record for periodic billing
#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "billings")]
pub struct Billing {
    #[domain(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: Decimal,
    pub total_tokens: i64,
    pub total_requests: i64,
    pub status: String,
    pub items: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

/// DTO for creating billing records
#[derive(Debug, Clone, Creatable)]
pub struct CreateBilling {
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: Decimal,
    pub total_tokens: i64,
    pub total_requests: i64,
    pub status: String,
    pub items: Option<JsonValue>,
}

/// Billing status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BillingStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
}

impl BillingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingStatus::Pending => "pending",
            BillingStatus::Paid => "paid",
            BillingStatus::Overdue => "overdue",
            BillingStatus::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for BillingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Billing item detail (stored in items JSONB)
#[derive(Debug, Clone, Serialize, Deserialize, Schematic)]
pub struct BillingItem {
    pub model: String,
    pub provider: String,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: f64,
}

/// Billing info for API responses
#[derive(Debug, Clone, Serialize, Schematic)]
pub struct BillingInfo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost: Decimal,
    pub total_tokens: i64,
    pub total_requests: i64,
    pub status: String,
    pub items: Option<Vec<BillingItem>>,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

impl From<Billing> for BillingInfo {
    fn from(b: Billing) -> Self {
        let items = b.items.and_then(|v| serde_json::from_value(v).ok());
        Self {
            id: b.id,
            user_id: b.user_id,
            period_start: b.period_start,
            period_end: b.period_end,
            total_cost: b.total_cost,
            total_tokens: b.total_tokens,
            total_requests: b.total_requests,
            status: b.status,
            items,
            created_at: b.created_at,
            paid_at: b.paid_at,
        }
    }
}

/// User balance for prepaid model (optional)
#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "user_balances")]
pub struct UserBalance {
    #[domain(primary_key)]
    pub user_id: Uuid,
    pub balance: Decimal,
    pub credit_limit: Decimal,
    pub lifetime_usage: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// User balance info for API responses
#[derive(Debug, Clone, Serialize, Schematic)]
pub struct UserBalanceInfo {
    pub user_id: Uuid,
    pub balance: Decimal,
    pub credit_limit: Decimal,
    pub lifetime_usage: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl From<UserBalance> for UserBalanceInfo {
    fn from(b: UserBalance) -> Self {
        Self {
            user_id: b.user_id,
            balance: b.balance,
            credit_limit: b.credit_limit,
            lifetime_usage: b.lifetime_usage,
            updated_at: b.updated_at,
        }
    }
}

/// Request to add balance to a user account
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct AddBalanceRequest {
    pub amount: Decimal,
}
