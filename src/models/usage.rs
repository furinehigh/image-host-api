use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UsageCounter {
    pub date: NaiveDate,
    pub api_key_id: Uuid,
    pub requests: i64,
    pub bytes_served: i64,
    pub uploads: i64,
}

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub key: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub total_requests: i64,
    pub total_bytes_served: i64,
    pub total_uploads: i64,
    pub daily_breakdown: Vec<DailyUsage>,
}

#[derive(Debug, Serialize)]
pub struct DailyUsage {
    pub date: NaiveDate,
    pub requests: i64,
    pub bytes_served: i64,
    pub uploads: i64,
}
