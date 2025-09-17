use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use image_hosting_server::{create_app, config::Config, database::Database};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_health_check() {
    let config = Config::from_env().expect("Failed to load config");
    let db = Database::new(&config.database_url).await.expect("Failed to connect to database");
    let app = create_app(Arc::new(db), config).await;

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_upload_image_unauthorized() {
    let config = Config::from_env().expect("Failed to load config");
    let db = Database::new(&config.database_url).await.expect("Failed to connect to database");
    let app = create_app(Arc::new(db), config).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upload")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_user_registration() {
    let config = Config::from_env().expect("Failed to load config");
    let db = Database::new(&config.database_url).await.expect("Failed to connect to database");
    let app = create_app(Arc::new(db), config).await;

    let user_data = json!({
        "username": format!("testuser_{}", Uuid::new_v4()),
        "email": format!("test_{}@example.com", Uuid::new_v4()),
        "password": "securepassword123"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(user_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = Config::from_env().expect("Failed to load config");
    let db = Database::new(&config.database_url).await.expect("Failed to connect to database");
    let app = create_app(Arc::new(db), config).await;

    let response = app
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_documentation() {
    let config = Config::from_env().expect("Failed to load config");
    let db = Database::new(&config.database_url).await.expect("Failed to connect to database");
    let app = create_app(Arc::new(db), config).await;

    let response = app
        .oneshot(Request::builder().uri("/docs/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
