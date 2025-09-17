use image_hosting_server::{database::Database, models::{User, Image, ApiKey}};
use sqlx::PgPool;
use uuid::Uuid;
use std::env;

async fn setup_test_db() -> Database {
    let database_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/image_hosting_test".to_string());
    
    let db = Database::new(&database_url).await.expect("Failed to connect to test database");
    db.migrate().await.expect("Failed to run migrations");
    
    // Clean up any existing test data
    sqlx::query!("TRUNCATE TABLE images, api_keys, users RESTART IDENTITY CASCADE")
        .execute(db.pool())
        .await
        .expect("Failed to clean test database");
    
    db
}

#[tokio::test]
async fn test_create_and_get_user() {
    let db = setup_test_db().await;
    
    let email = "test@example.com";
    let password_hash = "hashed_password";
    
    // Create user
    let created_user = db.create_user(email, password_hash).await.unwrap();
    assert_eq!(created_user.email, email);
    assert_eq!(created_user.password_hash, password_hash);
    assert_eq!(created_user.quota_bytes, 1073741824); // 1GB
    assert_eq!(created_user.used_bytes, 0);
    
    // Get user by email
    let retrieved_user = db.get_user_by_email(email).await.unwrap().unwrap();
    assert_eq!(retrieved_user.id, created_user.id);
    assert_eq!(retrieved_user.email, email);
    
    // Get user by ID
    let retrieved_by_id = db.get_user_by_id(created_user.id).await.unwrap().unwrap();
    assert_eq!(retrieved_by_id.id, created_user.id);
    assert_eq!(retrieved_by_id.email, email);
}

#[tokio::test]
async fn test_get_nonexistent_user() {
    let db = setup_test_db().await;
    
    let result = db.get_user_by_email("nonexistent@example.com").await.unwrap();
    assert!(result.is_none());
    
    let result = db.get_user_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_user_quota() {
    let db = setup_test_db().await;
    
    let user = db.create_user("test@example.com", "password").await.unwrap();
    
    // Update quota
    db.update_user_quota(user.id, 500000).await.unwrap();
    
    // Verify update
    let updated_user = db.get_user_by_id(user.id).await.unwrap().unwrap();
    assert_eq!(updated_user.used_bytes, 500000);
}

#[tokio::test]
async fn test_api_key_operations() {
    let db = setup_test_db().await;
    
    let user = db.create_user("test@example.com", "password").await.unwrap();
    let key_hash = "hashed_api_key";
    let key_name = "Test API Key";
    
    // Create API key
    let created_key = db.create_api_key(user.id, key_hash, key_name).await.unwrap();
    assert_eq!(created_key.user_id, user.id);
    assert_eq!(created_key.key_hash, key_hash);
    assert_eq!(created_key.name, key_name);
    
    // Get API key by hash
    let retrieved_key = db.get_api_key_by_hash(key_hash).await.unwrap().unwrap();
    assert_eq!(retrieved_key.id, created_key.id);
    
    // Update last used
    db.update_api_key_last_used(created_key.id).await.unwrap();
    
    // Get user API keys
    let user_keys = db.get_user_api_keys(user.id).await.unwrap();
    assert_eq!(user_keys.len(), 1);
    assert_eq!(user_keys[0].id, created_key.id);
    
    // Delete API key
    let deleted = db.delete_api_key(created_key.id, user.id).await.unwrap();
    assert!(deleted);
    
    // Verify deletion
    let result = db.get_api_key_by_hash(key_hash).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_image_operations() {
    let db = setup_test_db().await;
    
    let user = db.create_user("test@example.com", "password").await.unwrap();
    
    let image = Image {
        id: Uuid::new_v4(),
        user_id: user.id,
        filename: "test.jpg".to_string(),
        original_filename: "original_test.jpg".to_string(),
        mime_type: "image/jpeg".to_string(),
        file_size: 1024,
        width: 800,
        height: 600,
        sha256_hash: "abcdef123456".to_string(),
        storage_path: "/uploads/test.jpg".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Create image
    let created_image = db.create_image(&image).await.unwrap();
    assert_eq!(created_image.id, image.id);
    assert_eq!(created_image.filename, image.filename);
    
    // Get image by ID
    let retrieved_image = db.get_image_by_id(image.id).await.unwrap().unwrap();
    assert_eq!(retrieved_image.id, image.id);
    
    // Get image by hash
    let retrieved_by_hash = db.get_image_by_hash(&image.sha256_hash).await.unwrap().unwrap();
    assert_eq!(retrieved_by_hash.id, image.id);
    
    // Get user images
    let user_images = db.get_user_images(user.id, 10, 0).await.unwrap();
    assert_eq!(user_images.len(), 1);
    assert_eq!(user_images[0].id, image.id);
    
    // Get storage usage
    let usage = db.get_user_storage_usage(user.id).await.unwrap();
    assert_eq!(usage, 1024);
    
    // Delete image
    let deleted = db.delete_image(image.id, user.id).await.unwrap();
    assert!(deleted);
    
    // Verify deletion
    let result = db.get_image_by_id(image.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_records() {
    let db = setup_test_db().await;
    
    let user = db.create_user("test@example.com", "password").await.unwrap();
    
    // Try to delete non-existent API key
    let deleted = db.delete_api_key(Uuid::new_v4(), user.id).await.unwrap();
    assert!(!deleted);
    
    // Try to delete non-existent image
    let deleted = db.delete_image(Uuid::new_v4(), user.id).await.unwrap();
    assert!(!deleted);
}

#[tokio::test]
async fn test_user_images_pagination() {
    let db = setup_test_db().await;
    
    let user = db.create_user("test@example.com", "password").await.unwrap();
    
    // Create multiple images
    for i in 0..5 {
        let image = Image {
            id: Uuid::new_v4(),
            user_id: user.id,
            filename: format!("test_{}.jpg", i),
            original_filename: format!("original_test_{}.jpg", i),
            mime_type: "image/jpeg".to_string(),
            file_size: 1024,
            width: 800,
            height: 600,
            sha256_hash: format!("hash_{}", i),
            storage_path: format!("/uploads/test_{}.jpg", i),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        db.create_image(&image).await.unwrap();
    }
    
    // Test pagination
    let first_page = db.get_user_images(user.id, 2, 0).await.unwrap();
    assert_eq!(first_page.len(), 2);
    
    let second_page = db.get_user_images(user.id, 2, 2).await.unwrap();
    assert_eq!(second_page.len(), 2);
    
    let third_page = db.get_user_images(user.id, 2, 4).await.unwrap();
    assert_eq!(third_page.len(), 1);
}
