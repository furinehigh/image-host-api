use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod database;
mod auth;
mod handlers;
mod models;
mod services;
mod middleware;
mod storage;
mod errors;

use config::Config;
use database::Database;
use services::redis::RedisService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "image_hosting_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Starting image hosting server with config: {:?}", config);

    // Initialize database
    let database = Database::new(&config.database_url).await?;
    database.migrate().await?;

    // Initialize Redis
    let redis = RedisService::new(&config.redis_url).await?;

    // Initialize metrics
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()?;

    // Build application state
    let app_state = handlers::AppState {
        database,
        redis,
        config: config.clone(),
    };

    // Build router with rate limiting middleware
    let app = Router::new()
        .route("/health/live", get(handlers::health::liveness))
        .route("/health/ready", get(handlers::health::readiness))
        .route("/metrics", get(move || async move { 
            metrics_handle.render() 
        }))
        .route("/v1/uploads", post(handlers::images::upload_image))
        .route("/v1/images/:id", get(handlers::images::get_image))
        .route("/v1/images/:id", axum::routing::delete(handlers::images::delete_image))
        .route("/v1/images/:id/metadata", get(handlers::images::get_metadata))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::quota::quota_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        // Auth routes (no rate limiting needed for registration/login)
        .route("/v1/auth/register", post(handlers::auth::register))
        .route("/v1/auth/login", post(handlers::auth::login))
        .route("/v1/auth/refresh", post(handlers::auth::refresh))
        // Admin routes with different rate limits
        .route("/v1/admin/keys", post(handlers::admin::create_api_key))
        .route("/v1/admin/keys/:key", get(handlers::admin::get_api_key))
        .route("/v1/admin/keys/:key", axum::routing::delete(handlers::admin::revoke_api_key))
        .route("/v1/admin/usage", get(handlers::admin::get_usage))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit::admin_rate_limit_middleware,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(config.max_upload_size))
        )
        .with_state(app_state);

    // Add OpenAPI documentation
    let app = handlers::docs::add_docs(app);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
