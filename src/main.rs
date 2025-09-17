use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    routing::{get, post, delete},
    Router,
    middleware,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod database;
mod error;
mod handlers;
mod middleware;
use crate::middleware as app_middleware;
mod models;
mod services;
mod utils;

use config::Config;
use database::Database;
use services::redis::RedisService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "image_hosting_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    
    // Create upload directory
    utils::file::create_upload_directory(&config.upload_dir)?;
    
    // Initialize image processor
    services::image_processor::ImageProcessor::init()?;
    
    // Initialize database
    let database = Database::new(&config.database_url).await?;
    database.migrate().await?;
    
    // Initialize Redis
    let redis = RedisService::new(&config.redis_url).await?;
    
    // Build application state
    let app_state = handlers::AppState {
        database,
        redis,
        config: config.clone(),
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(Any);

    // Build the application router
    let app = Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/refresh", post(handlers::auth::refresh_token))
        .route("/api/v1/auth/api-keys", post(handlers::auth::create_api_key))
        .route("/api/v1/auth/api-keys", get(handlers::auth::list_api_keys))
        .route("/api/v1/auth/api-keys/:id", delete(handlers::auth::revoke_api_key))
        // Protected routes
        .route("/api/v1/upload", post(handlers::upload::upload_image))
        .route("/api/v1/images", get(handlers::images::list_user_images))
        .route("/api/v1/images/:id", get(handlers::images::get_image))
        .route("/api/v1/images/:id/transform", get(handlers::images::transform_image))
        .route("/api/v1/images/:id", delete(handlers::images::delete_image))
        .route("/api/v1/user/quota", get(handlers::user::get_quota))
        .route("/metrics", get(handlers::metrics::metrics))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors)
                .layer(DefaultBodyLimit::max(config.max_file_size))
                .layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    app_middleware::cache::cache_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    app_middleware::rate_limit::rate_limit_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    app_middleware::auth::auth_middleware,
                ))
        )
        .with_state(app_state);

    // Add OpenAPI documentation
    let app = app.merge(handlers::docs::create_docs_router());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
