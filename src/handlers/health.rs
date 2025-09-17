use axum::{extract::State, response::Json};
use serde_json::json;

use crate::{errors::Result, handlers::AppState};

pub async fn liveness() -> Result<Json<serde_json::Value>> {
    Ok(Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

pub async fn readiness(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").fetch_one(state.database.pool()).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    // Check Redis connection
    let redis_status = match redis::cmd("PING")
        .query_async::<_, String>(&mut state.redis.connection_manager().clone())
        .await
    {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let overall_status = if db_status == "healthy" && redis_status == "healthy" {
        "ready"
    } else {
        "not_ready"
    };

    Ok(Json(json!({
        "status": overall_status,
        "checks": {
            "database": db_status,
            "redis": redis_status
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
