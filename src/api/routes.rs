use crate::{
    api::handlers,
    auth::{auth_middleware, optional_auth_middleware},
    db::DatabaseService,
};
use axum::{
    middleware,
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;

/// Create the API router with all endpoints
pub fn create_router(db: Arc<DatabaseService>) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/user/me", get(handlers::get_current_user))
        .route("/api-keys", post(handlers::create_api_key))
        .route("/api-keys", get(handlers::list_api_keys))
        .route("/api-keys/:id", delete(handlers::revoke_api_key))
        .route("/usage", get(handlers::get_usage))
        .layer(middleware::from_fn_with_state(db.clone(), auth_middleware));

    // Combine routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(db)
}