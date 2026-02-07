use crate::db::DatabaseService;
use crate::models::api_key::ApiKey;
use crate::models::user::User;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::rate_limiter::RateLimiter;

/// Result of quota check
#[derive(Debug)]
pub enum QuotaCheckResult {
    /// Request is allowed to proceed
    Allowed,
    /// Rate limit exceeded, retry after specified seconds
    RateLimited { retry_after_secs: u64 },
    /// Monthly quota exceeded
    QuotaExceeded { limit: Decimal, used: Decimal },
    /// Model not in API key's allowed list
    ModelNotAllowed { model: String },
    /// Insufficient balance (for prepaid model)
    InsufficientBalance { balance: Decimal, required: Decimal },
}

/// Quota checker that validates rate limits and quotas before requests
pub struct QuotaChecker {
    rate_limiter: RateLimiter,
}

impl Default for QuotaChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaChecker {
    pub fn new() -> Self {
        Self {
            rate_limiter: RateLimiter::new(),
        }
    }

    /// Perform all quota checks before processing a request
    pub async fn check_quota(
        &self,
        user: &User,
        model: &str,
        api_key: Option<&ApiKey>,
        db: &DatabaseService,
    ) -> QuotaCheckResult {
        // 1. Check model allowlist (if API key has restrictions)
        if let Some(key) = api_key {
            if !key.is_model_allowed(model) {
                return QuotaCheckResult::ModelNotAllowed {
                    model: model.to_string(),
                };
            }
        }

        // 2. Check user-level rate limit (RPM)
        if let Some(limit_rpm) = user.rate_limit_rpm {
            if let Err(retry_after) = self.rate_limiter.check_and_record(user.id, limit_rpm) {
                return QuotaCheckResult::RateLimited {
                    retry_after_secs: retry_after,
                };
            }
        }

        // 3. Check user monthly quota
        if let Some(monthly_quota) = user.monthly_quota {
            if user.current_month_usage >= monthly_quota {
                return QuotaCheckResult::QuotaExceeded {
                    limit: monthly_quota,
                    used: user.current_month_usage,
                };
            }
        }

        // 4. Balance check for prepaid model (required)
        // All users should have a balance record (created at registration)
        match db.get_user_balance(user.id).await {
            Ok(Some(balance)) => {
                if balance.balance <= Decimal::ZERO && balance.credit_limit <= Decimal::ZERO {
                    return QuotaCheckResult::InsufficientBalance {
                        balance: balance.balance,
                        required: Decimal::ZERO,
                    };
                }
            }
            Ok(None) => {
                // Defensive: should not happen as balance is created with user
                tracing::error!("User {} has no balance record", user.id);
                return QuotaCheckResult::InsufficientBalance {
                    balance: Decimal::ZERO,
                    required: Decimal::ZERO,
                };
            }
            Err(e) => {
                tracing::warn!("Failed to check user balance: {}", e);
                // On DB error, fail closed (reject)
                return QuotaCheckResult::InsufficientBalance {
                    balance: Decimal::ZERO,
                    required: Decimal::ZERO,
                };
            }
        }

        QuotaCheckResult::Allowed
    }

    /// Record a request for rate limiting purposes (use when check was done separately)
    pub fn record_request(&self, user_id: Uuid, limit_rpm: i32) {
        let _ = self.rate_limiter.check_and_record(user_id, limit_rpm);
    }

    /// Cleanup old rate limit entries (call periodically)
    pub fn cleanup(&self) {
        self.rate_limiter.cleanup();
    }

    /// Get the underlying rate limiter for direct access if needed
    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}
