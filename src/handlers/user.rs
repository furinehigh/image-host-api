use axum::{
    extract::{State, Request},
    response::Json,
    http::StatusCode,
};
use uuid::Uuid;
use crate::{
    handlers::AppState,
    models::UserQuotaResponse,
    error::{AppError, Result},
    middleware::auth::{get_authenticated_user, AuthenticatedUser},
};

pub async fn get_quota(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<UserQuotaResponse>> {
    let user = get_authenticated_user(&request)?;
    let user_id = user.user_id;

    let user = state.database.get_user_by_id(user_id).await?
        .ok_or(AppError::Unauthorized)?;

    let current_usage = state.database.get_user_storage_usage(user_id).await?;

    let response = UserQuotaResponse {
        quota_bytes: user.quota_bytes,
        used_bytes: current_usage,
        remaining_bytes: (user.quota_bytes - current_usage).max(0),
    };

    Ok(Json(response))
}
