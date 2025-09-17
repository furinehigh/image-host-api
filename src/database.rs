use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use crate::models::{Image, User, ApiKey};
use crate::error::Result;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;

        Ok(Database { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // User operations
    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash, quota_bytes, used_bytes)
            VALUES ($1, $2, $3, 0)
            RETURNING id, email, password_hash, quota_bytes, used_bytes, created_at, updated_at
            "#,
            email,
            password_hash,
            1073741824i64 // 1GB default quota
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, quota_bytes, used_bytes, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, quota_bytes, used_bytes, created_at, updated_at FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn update_user_quota(&self, user_id: Uuid, used_bytes: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE users SET used_bytes = $1, updated_at = NOW() WHERE id = $2",
            used_bytes,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // API Key operations
    pub async fn create_api_key(&self, user_id: Uuid, key_hash: &str, name: &str) -> Result<ApiKey> {
        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            INSERT INTO api_keys (user_id, key_hash, name)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, key_hash, name, last_used_at, created_at, expires_at
            "#,
            user_id,
            key_hash,
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(api_key)
    }

    pub async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
        let api_key = sqlx::query_as!(
            ApiKey,
            "SELECT id, user_id, key_hash, name, last_used_at, created_at, expires_at FROM api_keys WHERE key_hash = $1",
            key_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(api_key)
    }

    pub async fn update_api_key_last_used(&self, api_key_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET last_used_at = NOW() WHERE id = $1",
            api_key_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let api_keys = sqlx::query_as!(
            ApiKey,
            "SELECT id, user_id, key_hash, name, last_used_at, created_at, expires_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(api_keys)
    }

    pub async fn delete_api_key(&self, key_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM api_keys WHERE id = $1 AND user_id = $2",
            key_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // Image operations
    pub async fn create_image(&self, image: &Image) -> Result<Image> {
        let created_image = sqlx::query_as!(
            Image,
            r#"
            INSERT INTO images (id, user_id, filename, original_filename, mime_type, file_size, width, height, sha256_hash, storage_path)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, filename, original_filename, mime_type, file_size, width, height, sha256_hash, storage_path, created_at, updated_at
            "#,
            image.id,
            image.user_id,
            image.filename,
            image.original_filename,
            image.mime_type,
            image.file_size,
            image.width,
            image.height,
            image.sha256_hash,
            image.storage_path
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(created_image)
    }

    pub async fn get_image_by_id(&self, image_id: Uuid) -> Result<Option<Image>> {
        let image = sqlx::query_as!(
            Image,
            "SELECT id, user_id, filename, original_filename, mime_type, file_size, width, height, sha256_hash, storage_path, created_at, updated_at FROM images WHERE id = $1",
            image_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(image)
    }

    pub async fn get_image_by_hash(&self, sha256_hash: &str) -> Result<Option<Image>> {
        let image = sqlx::query_as!(
            Image,
            "SELECT id, user_id, filename, original_filename, mime_type, file_size, width, height, sha256_hash, storage_path, created_at, updated_at FROM images WHERE sha256_hash = $1",
            sha256_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(image)
    }

    pub async fn get_user_images(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Image>> {
        let images = sqlx::query_as!(
            Image,
            "SELECT id, user_id, filename, original_filename, mime_type, file_size, width, height, sha256_hash, storage_path, created_at, updated_at FROM images WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(images)
    }

    pub async fn delete_image(&self, image_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM images WHERE id = $1 AND user_id = $2",
            image_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_user_storage_usage(&self, user_id: Uuid) -> Result<i64> {
        let result = sqlx::query!(
            "SELECT COALESCE(SUM(file_size), 0) as total_size FROM images WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.total_size.unwrap_or(0))
    }
}
