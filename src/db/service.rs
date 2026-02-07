use crate::models::{
    api_key::{ApiKey, CreateApiKey},
    billing::{Billing, CreateBilling, UserBalance},
    usage::{Usage, CreateUsage},
    user::{User, CreateUser},
    provider_config::{ProviderConfig, CreateProviderConfig, UsageStats, ModelUsageStats, ProviderUsageStats},
    model::{Model, CreateModel},
};
use chrono::{DateTime, Utc};
use conservator::{Creatable, Domain, Executor, Migrator, PooledConnection};
use rust_decimal::Decimal;
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
        let migrator = Migrator::from_path("./migrations")?;

        let mut conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        migrator.run(&mut conn).await?;

        tracing::info!("Migrations completed successfully");
        Ok(())
    }

    // User operations

    /// Create a new user with initial balance record (transactional)
    pub async fn create_user(&self, create_user: CreateUser) -> crate::Result<User> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Use CTE to atomically create user and balance in a single statement
        let query = r#"
            WITH new_user AS (
                INSERT INTO users (email, username, password_hash, is_active, is_admin, user_groups)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
            )
            INSERT INTO user_balances (user_id)
            SELECT id FROM new_user
            RETURNING (SELECT id FROM new_user)
        "#;

        let row = conn
            .query_one(
                query,
                &[
                    &create_user.email,
                    &create_user.username,
                    &create_user.password_hash,
                    &create_user.is_active,
                    &create_user.is_admin,
                    &create_user.user_groups,
                ],
            )
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        let user_id: Uuid = row.get(0);

        User::fetch_one_by_pk(&user_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Find a user by email
    pub async fn find_user_by_email(&self, email: &str) -> crate::Result<Option<User>> {
        User::select()
            .filter(User::COLUMNS.email.eq(email.to_string()))
            .optional(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Find a user by ID
    pub async fn find_user_by_id(&self, id: Uuid) -> crate::Result<Option<User>> {
        match User::fetch_one_by_pk(&id, &*self.pool).await {
            Ok(user) => Ok(Some(user)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// Update user last login
    pub async fn update_user_last_login(&self, user_id: Uuid) -> crate::Result<()> {
        let now = Utc::now();

        User::update()
            .set(User::COLUMNS.last_login, Some(now))
            .set(User::COLUMNS.updated_at, now)
            .filter(User::COLUMNS.id.eq(user_id))
            .execute(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // API Key operations

    /// Create a new API key
    pub async fn create_api_key(&self, create_api_key: CreateApiKey) -> crate::Result<ApiKey> {
        let api_key_id = create_api_key
            .insert::<ApiKey>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        ApiKey::fetch_one_by_pk(&api_key_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Find an API key by its value
    pub async fn find_api_key(&self, key: &str) -> crate::Result<Option<ApiKey>> {
        let api_key = ApiKey::select()
            .filter(
                ApiKey::COLUMNS.key.eq(key.to_string())
                    & ApiKey::COLUMNS.is_active.eq(true)
            )
            .optional(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Check if valid (not expired)
        if let Some(ref k) = api_key {
            if !k.is_valid() {
                return Ok(None);
            }
        }

        Ok(api_key)
    }

    /// Get an API key by its ID
    pub async fn get_api_key_by_id(&self, id: Uuid) -> crate::Result<Option<ApiKey>> {
        match ApiKey::fetch_one_by_pk(&id, &*self.pool).await {
            Ok(api_key) => {
                if api_key.is_valid() && api_key.is_active {
                    Ok(Some(api_key))
                } else {
                    Ok(None)
                }
            }
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// List all API keys for a user
    pub async fn list_user_api_keys(&self, user_id: Uuid) -> crate::Result<Vec<ApiKey>> {
        ApiKey::select()
            .filter(ApiKey::COLUMNS.user_id.eq(user_id))
            .order_by(ApiKey::COLUMNS.created_at.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Update API key last used
    pub async fn update_api_key_last_used(&self, api_key_id: Uuid) -> crate::Result<()> {
        let now = Utc::now();

        ApiKey::update()
            .set(ApiKey::COLUMNS.last_used, Some(now))
            .set(ApiKey::COLUMNS.updated_at, now)
            .filter(ApiKey::COLUMNS.id.eq(api_key_id))
            .execute(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    /// Delete an API key
    pub async fn delete_api_key(&self, api_key_id: Uuid, user_id: Uuid) -> crate::Result<()> {
        ApiKey::delete()
            .filter(
                ApiKey::COLUMNS.id.eq(api_key_id)
                    & ApiKey::COLUMNS.user_id.eq(user_id)
            )
            .execute(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // Usage operations

    /// Record API usage
    pub async fn record_usage(&self, create_usage: CreateUsage) -> crate::Result<()> {
        // Insert and ignore the returned ID
        let _ = create_usage
            .insert::<Usage>()
            .returning_pk(&*self.pool)
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
        Usage::select()
            .filter(
                Usage::COLUMNS.user_id.eq(user_id)
                    & Usage::COLUMNS.created_at.gte(start)
                    & Usage::COLUMNS.created_at.lte(end)
            )
            .order_by(Usage::COLUMNS.created_at.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    // Provider configuration operations

    /// Create a new provider configuration
    pub async fn create_provider_config(&self, create_config: CreateProviderConfig) -> crate::Result<ProviderConfig> {
        let config_id = create_config
            .insert::<ProviderConfig>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        ProviderConfig::fetch_one_by_pk(&config_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// List provider configs accessible by the given user groups
    /// Returns providers where allowed_groups is empty (public) or has intersection with user_groups
    pub async fn list_providers_for_groups(&self, user_groups: &[String]) -> crate::Result<Vec<ProviderConfig>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Query providers where allowed_groups is empty OR overlaps with user_groups
        let query = r#"
            SELECT * FROM provider_configs
            WHERE is_active = true
              AND (allowed_groups = '{}' OR allowed_groups && $1)
            ORDER BY priority DESC, created_at DESC
        "#;

        let rows = conn
            .query(query, &[&user_groups])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        let configs: Vec<ProviderConfig> = rows
            .iter()
            .map(|row| ProviderConfig {
                id: row.get("id"),
                provider_type: row.get("provider_type"),
                name: row.get("name"),
                api_key: row.get("api_key"),
                base_url: row.get("base_url"),
                priority: row.get("priority"),
                is_active: row.get("is_active"),
                rate_limit: row.get("rate_limit"),
                monthly_quota: row.get("monthly_quota"),
                used_quota: row.get("used_quota"),
                allowed_groups: row.get("allowed_groups"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(configs)
    }

    /// List all provider configs (admin only)
    pub async fn list_all_provider_configs(&self) -> crate::Result<Vec<ProviderConfig>> {
        ProviderConfig::select()
            .order_by(ProviderConfig::COLUMNS.priority.desc())
            .order_by(ProviderConfig::COLUMNS.created_at.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Get a provider config by ID
    pub async fn get_provider_config(&self, id: Uuid) -> crate::Result<Option<ProviderConfig>> {
        match ProviderConfig::fetch_one_by_pk(&id, &*self.pool).await {
            Ok(config) => Ok(Some(config)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// Update provider config using builder pattern
    pub async fn update_provider_config(
        &self,
        id: Uuid,
        name: Option<String>,
        api_key: Option<String>,
        base_url: Option<Option<String>>,
        priority: Option<i32>,
        is_active: Option<bool>,
        rate_limit: Option<Option<i32>>,
        monthly_quota: Option<Option<Decimal>>,
        allowed_groups: Option<Vec<String>>,
    ) -> crate::Result<ProviderConfig> {
        // Fetch the current config, then update it
        let mut config = ProviderConfig::fetch_one_by_pk(&id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Update fields that were provided
        if let Some(n) = name {
            config.name = n;
        }
        if let Some(key) = api_key {
            config.api_key = key;
        }
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(p) = priority {
            config.priority = p;
        }
        if let Some(active) = is_active {
            config.is_active = active;
        }
        if let Some(limit) = rate_limit {
            config.rate_limit = limit;
        }
        if let Some(quota) = monthly_quota {
            config.monthly_quota = quota;
        }
        if let Some(groups) = allowed_groups {
            config.allowed_groups = groups;
        }

        config.updated_at = Utc::now();

        // Save the updated config
        config.save(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(config)
    }

    /// Delete provider config (admin only, no user_id check needed)
    pub async fn delete_provider_config(&self, id: Uuid) -> crate::Result<()> {
        ProviderConfig::delete_by_pk(&id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // Model operations

    /// Create a new model under a provider
    pub async fn create_model(&self, create_model: CreateModel) -> crate::Result<Model> {
        let model_id = create_model
            .insert::<Model>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Model::fetch_one_by_pk(&model_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// List models for a provider
    pub async fn list_models_for_provider(&self, provider_config_id: Uuid) -> crate::Result<Vec<Model>> {
        Model::select()
            .filter(Model::COLUMNS.provider_config_id.eq(provider_config_id))
            .order_by(Model::COLUMNS.model_id.asc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// List all active models accessible by user groups
    pub async fn list_models_for_groups(&self, user_groups: &[String]) -> crate::Result<Vec<Model>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Query models where the associated provider is active and accessible
        let query = r#"
            SELECT m.* FROM models m
            INNER JOIN provider_configs p ON m.provider_config_id = p.id
            WHERE m.is_active = true
              AND p.is_active = true
              AND (p.allowed_groups = '{}' OR p.allowed_groups && $1)
            ORDER BY p.priority DESC, m.model_id ASC
        "#;

        let rows = conn
            .query(query, &[&user_groups])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        let models: Vec<Model> = rows
            .iter()
            .map(|row| Model {
                id: row.get("id"),
                provider_config_id: row.get("provider_config_id"),
                model_id: row.get("model_id"),
                display_name: row.get("display_name"),
                input_cost_per_1k: row.get("input_cost_per_1k"),
                output_cost_per_1k: row.get("output_cost_per_1k"),
                max_context: row.get("max_context"),
                supports_tools: row.get("supports_tools"),
                supports_vision: row.get("supports_vision"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(models)
    }

    /// Get a model by ID
    pub async fn get_model(&self, id: Uuid) -> crate::Result<Option<Model>> {
        match Model::fetch_one_by_pk(&id, &*self.pool).await {
            Ok(model) => Ok(Some(model)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// Get a model by model_id string (e.g., "gpt-4o")
    pub async fn get_model_by_model_id(&self, model_id: &str) -> crate::Result<Option<Model>> {
        Model::select()
            .filter(Model::COLUMNS.model_id.eq(model_id.to_string()))
            .optional(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Update a model
    pub async fn update_model(
        &self,
        id: Uuid,
        display_name: Option<Option<String>>,
        input_cost_per_1k: Option<Option<Decimal>>,
        output_cost_per_1k: Option<Option<Decimal>>,
        max_context: Option<Option<i32>>,
        supports_tools: Option<bool>,
        supports_vision: Option<bool>,
        is_active: Option<bool>,
    ) -> crate::Result<Model> {
        let mut model = Model::fetch_one_by_pk(&id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        if let Some(name) = display_name {
            model.display_name = name;
        }
        if let Some(cost) = input_cost_per_1k {
            model.input_cost_per_1k = cost;
        }
        if let Some(cost) = output_cost_per_1k {
            model.output_cost_per_1k = cost;
        }
        if let Some(ctx) = max_context {
            model.max_context = ctx;
        }
        if let Some(tools) = supports_tools {
            model.supports_tools = tools;
        }
        if let Some(vision) = supports_vision {
            model.supports_vision = vision;
        }
        if let Some(active) = is_active {
            model.is_active = active;
        }

        model.updated_at = Utc::now();

        model.save(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(model)
    }

    /// Delete a model
    pub async fn delete_model(&self, id: Uuid) -> crate::Result<()> {
        Model::delete_by_pk(&id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    // User group operations

    /// Update user groups
    pub async fn update_user_groups(&self, user_id: Uuid, user_groups: Vec<String>) -> crate::Result<User> {
        let mut user = User::fetch_one_by_pk(&user_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        user.user_groups = user_groups;
        user.updated_at = Utc::now();

        user.save(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(user)
    }

    /// List all users (admin only)
    pub async fn list_all_users(&self) -> crate::Result<Vec<User>> {
        User::select()
            .order_by(User::COLUMNS.created_at.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Get usage statistics for a user
    /// Uses CTE to fetch all stats in two optimized queries instead of three
    pub async fn get_usage_stats(
        &self,
        user_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> crate::Result<UsageStats> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Query 1: Get totals and by_model in single query using CTE
        let combined_query = r#"
            WITH filtered_usage AS (
                SELECT * FROM usage
                WHERE user_id = $1 AND created_at >= $2 AND created_at <= $3
            ),
            totals AS (
                SELECT
                    COUNT(*)::bigint as total_requests,
                    COALESCE(SUM(total_tokens), 0)::bigint as total_tokens,
                    COALESCE(SUM(cost), 0.0)::float8 as total_cost
                FROM filtered_usage
            ),
            by_model AS (
                SELECT
                    model,
                    COUNT(*)::bigint as requests,
                    COALESCE(SUM(prompt_tokens), 0)::bigint as prompt_tokens,
                    COALESCE(SUM(completion_tokens), 0)::bigint as completion_tokens,
                    COALESCE(SUM(total_tokens), 0)::bigint as total_tokens,
                    COALESCE(SUM(cost), 0.0)::float8 as cost
                FROM filtered_usage
                GROUP BY model
                ORDER BY cost DESC
            )
            SELECT
                'total' as stat_type,
                NULL as name,
                t.total_requests as requests,
                t.total_tokens as total_tokens,
                0::bigint as prompt_tokens,
                0::bigint as completion_tokens,
                t.total_cost as cost
            FROM totals t
            UNION ALL
            SELECT
                'model' as stat_type,
                model as name,
                requests,
                total_tokens,
                prompt_tokens,
                completion_tokens,
                cost
            FROM by_model
        "#;

        let combined_rows = conn
            .query(combined_query, &[&user_id, &start, &end])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        let mut total_requests: i64 = 0;
        let mut total_tokens: i64 = 0;
        let mut total_cost: f64 = 0.0;
        let mut by_model: Vec<ModelUsageStats> = Vec::new();

        for row in combined_rows {
            let stat_type: String = row.get("stat_type");
            if stat_type == "total" {
                total_requests = row.get("requests");
                total_tokens = row.get("total_tokens");
                total_cost = row.get("cost");
            } else {
                by_model.push(ModelUsageStats {
                    model: row.get("name"),
                    requests: row.get("requests"),
                    prompt_tokens: row.get("prompt_tokens"),
                    completion_tokens: row.get("completion_tokens"),
                    total_tokens: row.get("total_tokens"),
                    cost: row.get("cost"),
                });
            }
        }

        // Query 2: Get by_provider
        let provider_query = r#"
            SELECT
                provider,
                COUNT(*)::bigint as requests,
                COALESCE(SUM(total_tokens), 0)::bigint as total_tokens,
                COALESCE(SUM(cost), 0.0)::float8 as cost
            FROM usage
            WHERE user_id = $1 AND created_at >= $2 AND created_at <= $3
            GROUP BY provider
            ORDER BY cost DESC
        "#;

        let provider_rows = conn
            .query(provider_query, &[&user_id, &start, &end])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        let by_provider: Vec<ProviderUsageStats> = provider_rows
            .iter()
            .map(|r| ProviderUsageStats {
                provider: r.get("provider"),
                requests: r.get("requests"),
                total_tokens: r.get("total_tokens"),
                cost: r.get("cost"),
            })
            .collect();

        Ok(UsageStats {
            user_id,
            total_requests,
            total_tokens,
            total_cost,
            by_model,
            by_provider,
        })
    }

    // Billing operations

    /// Create a new billing record
    pub async fn create_billing(&self, create_billing: CreateBilling) -> crate::Result<Billing> {
        let billing_id = create_billing
            .insert::<Billing>()
            .returning_pk(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Billing::fetch_one_by_pk(&billing_id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// List all billings for a user
    pub async fn list_user_billings(&self, user_id: Uuid) -> crate::Result<Vec<Billing>> {
        Billing::select()
            .filter(Billing::COLUMNS.user_id.eq(user_id))
            .order_by(Billing::COLUMNS.period_start.desc())
            .all(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))
    }

    /// Get a billing record by ID
    pub async fn get_billing(&self, id: Uuid) -> crate::Result<Option<Billing>> {
        match Billing::fetch_one_by_pk(&id, &*self.pool).await {
            Ok(billing) => Ok(Some(billing)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// Update billing status
    pub async fn update_billing_status(
        &self,
        id: Uuid,
        status: &str,
        paid_at: Option<DateTime<Utc>>,
    ) -> crate::Result<Billing> {
        let mut billing = Billing::fetch_one_by_pk(&id, &*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        billing.status = status.to_string();
        if let Some(paid) = paid_at {
            billing.paid_at = Some(paid);
        }

        billing.save(&*self.pool)
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(billing)
    }

    /// Update user's monthly usage (add to current_month_usage)
    pub async fn update_user_monthly_usage(&self, user_id: Uuid, cost: Decimal) -> crate::Result<()> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        // Atomically add to current_month_usage
        let query = r#"
            UPDATE users
            SET current_month_usage = current_month_usage + $2,
                updated_at = NOW()
            WHERE id = $1
        "#;

        conn.execute(query, &[&user_id, &cost])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(())
    }

    /// Reset monthly usage for all users (call at month start)
    pub async fn reset_monthly_usage(&self) -> crate::Result<u64> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            UPDATE users
            SET current_month_usage = 0,
                usage_reset_at = NOW(),
                updated_at = NOW()
        "#;

        let rows = conn.execute(query, &[])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(rows)
    }

    /// Check if billing already exists for a period
    pub async fn billing_exists(&self, user_id: Uuid, period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> crate::Result<bool> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            SELECT EXISTS(
                SELECT 1 FROM billings
                WHERE user_id = $1 AND period_start = $2 AND period_end = $3
            )
        "#;

        let row = conn.query_one(query, &[&user_id, &period_start, &period_end])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(row.get(0))
    }

    // User balance operations (prepaid model)

    /// Get user balance (for prepaid model)
    pub async fn get_user_balance(&self, user_id: Uuid) -> crate::Result<Option<UserBalance>> {
        match UserBalance::fetch_one_by_pk(&user_id, &*self.pool).await {
            Ok(balance) => Ok(Some(balance)),
            Err(conservator::Error::TooManyRows(0)) => Ok(None),
            Err(e) => Err(crate::MantiError::Database(e)),
        }
    }

    /// Deduct from user balance (atomic operation)
    /// Returns the new balance after deduction, or None if user has no balance record
    pub async fn deduct_balance(&self, user_id: Uuid, amount: Decimal) -> crate::Result<Option<Decimal>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            UPDATE user_balances
            SET balance = balance - $2,
                lifetime_usage = lifetime_usage + $2,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING balance
        "#;

        let rows = conn.query(query, &[&user_id, &amount])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(rows.first().map(|r| r.get(0)))
    }

    /// Add to user balance (admin operation for top-up)
    /// Returns the updated UserBalance, or None if user has no balance record
    pub async fn add_balance(&self, user_id: Uuid, amount: Decimal) -> crate::Result<Option<UserBalance>> {
        let conn = self.pool.get().await
            .map_err(|e| crate::MantiError::Database(e))?;

        let query = r#"
            UPDATE user_balances
            SET balance = balance + $2,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING user_id, balance, credit_limit, lifetime_usage, updated_at
        "#;

        let rows = conn.query(query, &[&user_id, &amount])
            .await
            .map_err(|e| crate::MantiError::Database(e))?;

        Ok(rows.first().map(|r| UserBalance {
            user_id: r.get(0),
            balance: r.get(1),
            credit_limit: r.get(2),
            lifetime_usage: r.get(3),
            updated_at: r.get(4),
        }))
    }
}