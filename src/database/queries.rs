use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};
use crate::models::*;
use crate::errors::Result;

pub struct UserQueries;

impl UserQueries {
    pub async fn create_user(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
    ) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id, email, password_hash, created_at, is_admin, profile_json
            "#,
            email,
            password_hash
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, is_admin, profile_json FROM users WHERE email = $1",
            email
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, is_admin, profile_json FROM users WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }
}

pub struct ApiKeyQueries;

impl ApiKeyQueries {
    pub async fn create_api_key(
        pool: &PgPool,
        key_hash: &str,
        owner_id: Uuid,
        name: &str,
        limits: &ApiKeyLimits,
    ) -> Result<ApiKey> {
        let limits_json = serde_json::to_value(limits)?;
        
        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            INSERT INTO api_keys (key_hash, owner_id, name, limits_json)
            VALUES ($1, $2, $3, $4)
            RETURNING id, key_hash, owner_id, name, created_at, revoked_at, config_json, limits_json
            "#,
            key_hash,
            owner_id,
            name,
            limits_json
        )
        .fetch_one(pool)
        .await?;

        Ok(api_key)
    }

    pub async fn find_by_key_hash(pool: &PgPool, key_hash: &str) -> Result<Option<ApiKey>> {
        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
            SELECT id, key_hash, owner_id, name, created_at, revoked_at, config_json, limits_json
            FROM api_keys 
            WHERE key_hash = $1 AND revoked_at IS NULL
            "#,
            key_hash
        )
        .fetch_optional(pool)
        .await?;

        Ok(api_key)
    }

    pub async fn revoke_api_key(pool: &PgPool, id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET revoked_at = NOW() WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn check_limits(pool: &PgPool, api_key_id: Uuid) -> Result<(bool, String, i64, i64)> {
        let row = sqlx::query!(
            "SELECT * FROM check_api_key_limits($1)",
            api_key_id
        )
        .fetch_one(pool)
        .await?;

        Ok((
            row.exceeded.unwrap_or(true),
            row.limit_type.unwrap_or_default(),
            row.current_usage.unwrap_or(0),
            row.limit_value.unwrap_or(0),
        ))
    }
}

pub struct ImageQueries;

impl ImageQueries {
    pub async fn create_image(
        pool: &PgPool,
        owner_id: Uuid,
        sha256: &str,
        mime: &str,
        size_bytes: i64,
        width: i32,
        height: i32,
        storage_path: &str,
        is_public: bool,
    ) -> Result<Image> {
        let image = sqlx::query_as!(
            Image,
            r#"
            INSERT INTO images (owner_id, sha256, mime, orig_size_bytes, width, height, storage_path, is_public)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, owner_id, sha256, mime, orig_size_bytes, width, height, storage_path, variants, is_public, created_at, deleted_at
            "#,
            owner_id,
            sha256,
            mime,
            size_bytes,
            width,
            height,
            storage_path,
            is_public
        )
        .fetch_one(pool)
        .await?;

        Ok(image)
    }

    pub async fn find_by_sha256(pool: &PgPool, sha256: &str) -> Result<Option<Image>> {
        let image = sqlx::query_as!(
            Image,
            r#"
            SELECT id, owner_id, sha256, mime, orig_size_bytes, width, height, storage_path, variants, is_public, created_at, deleted_at
            FROM images 
            WHERE sha256 = $1 AND deleted_at IS NULL
            "#,
            sha256
        )
        .fetch_optional(pool)
        .await?;

        Ok(image)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Image>> {
        let image = sqlx::query_as!(
            Image,
            r#"
            SELECT id, owner_id, sha256, mime, orig_size_bytes, width, height, storage_path, variants, is_public, created_at, deleted_at
            FROM images 
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(image)
    }

    pub async fn update_variants(
        pool: &PgPool,
        id: Uuid,
        variants: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE images SET variants = $1 WHERE id = $2",
            variants,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE images SET deleted_at = NOW() WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn list_by_owner(
        pool: &PgPool,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Image>> {
        let images = sqlx::query_as!(
            Image,
            r#"
            SELECT id, owner_id, sha256, mime, orig_size_bytes, width, height, storage_path, variants, is_public, created_at, deleted_at
            FROM images 
            WHERE owner_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            owner_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok(images)
    }
}

pub struct UsageQueries;

impl UsageQueries {
    pub async fn update_usage(
        pool: &PgPool,
        api_key_id: Uuid,
        requests: i32,
        bytes_served: i64,
        uploads: i32,
    ) -> Result<()> {
        sqlx::query!(
            "SELECT update_usage_counter($1, $2, $3, $4)",
            api_key_id,
            requests,
            bytes_served,
            uploads
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_usage_stats(
        pool: &PgPool,
        api_key_id: Option<Uuid>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<UsageResponse> {
        let from = from_date.unwrap_or_else(|| {
            chrono::Utc::now().date_naive() - chrono::Duration::days(30)
        });
        let to = to_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let query = if let Some(key_id) = api_key_id {
            sqlx::query!(
                r#"
                SELECT date, requests, bytes_served, uploads
                FROM usage_counters
                WHERE api_key_id = $1 AND date BETWEEN $2 AND $3
                ORDER BY date DESC
                "#,
                key_id,
                from,
                to
            )
        } else {
            sqlx::query!(
                r#"
                SELECT date, SUM(requests) as requests, SUM(bytes_served) as bytes_served, SUM(uploads) as uploads
                FROM usage_counters
                WHERE date BETWEEN $1 AND $2
                GROUP BY date
                ORDER BY date DESC
                "#,
                from,
                to
            )
        };

        let rows = query.fetch_all(pool).await?;

        let mut total_requests = 0i64;
        let mut total_bytes_served = 0i64;
        let mut total_uploads = 0i64;
        let mut daily_breakdown = Vec::new();

        for row in rows {
            let requests = row.requests.unwrap_or(0);
            let bytes_served = row.bytes_served.unwrap_or(0);
            let uploads = row.uploads.unwrap_or(0);

            total_requests += requests;
            total_bytes_served += bytes_served;
            total_uploads += uploads;

            daily_breakdown.push(DailyUsage {
                date: row.date,
                requests,
                bytes_served,
                uploads,
            });
        }

        Ok(UsageResponse {
            total_requests,
            total_bytes_served,
            total_uploads,
            daily_breakdown,
        })
    }
}
