use crate::{
    api::{handlers, admin},
    auth::auth_middleware,
    db::DatabaseService,
};
use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;

/// Create the API router with all endpoints
pub fn create_router(db: Arc<DatabaseService>) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/refresh", post(handlers::refresh_token));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        // User profile
        .route("/user/me", get(handlers::get_current_user))
        .route("/user/profile", get(handlers::get_current_user))
        .route("/user/profile", put(handlers::update_profile))
        // Auth
        .route("/auth/validate", get(handlers::validate_token))
        .route("/auth/logout", post(handlers::logout))
        // API keys
        .route("/api-keys", post(handlers::create_api_key))
        .route("/api-keys", get(handlers::list_api_keys))
        .route("/api-keys/:id", delete(handlers::revoke_api_key))
        // Usage stats
        .route("/usage", get(handlers::get_usage))
        .route("/usage/stats", get(handlers::get_usage_stats))
        .layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // Admin routes (auth required + admin check)
    let admin_routes = Router::new()
        // Provider management (admin only, global providers)
        .route("/admin/providers", get(admin::list_all_providers))
        .route("/admin/providers", post(admin::create_provider))
        .route("/admin/providers/:id", put(admin::update_provider))
        .route("/admin/providers/:id", delete(admin::delete_provider))
        // Model management (admin only)
        .route("/admin/providers/:provider_id/models", get(admin::list_provider_models))
        .route("/admin/providers/:provider_id/models", post(admin::create_model))
        .route("/admin/models/:id", put(admin::update_model))
        .route("/admin/models/:id", delete(admin::delete_model))
        // User management (admin only)
        .route("/admin/users", get(admin::list_users))
        .route("/admin/users/:user_id/groups", put(admin::update_user_groups))
        .route("/admin/users/:user_id/usage", get(admin::get_usage_stats))
        .layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // Combine routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(db)
}
