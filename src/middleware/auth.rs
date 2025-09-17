use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{ApiKeyService, JwtService},
    database::queries::{ApiKeyQueries, UserQueries},
    errors::{AppError, Result},
    handlers::AppState,
    models::{ApiKey, User},
};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedApiKey {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub limits: serde_json::Value,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Try to get JWT token from Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        if let Some(auth_header) = auth_header {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                // Verify JWT token
                let jwt_service = JwtService::new(&state.config.jwt_secret);
                match jwt_service.verify_access_token(token) {
                    Ok(claims) => {
                        let user_id = Uuid::parse_str(&claims.sub)
                            .map_err(|_| {
                                (
                                    StatusCode::UNAUTHORIZED,
                                    Json(json!({"error": "Invalid token"})),
                                ).into_response()
                            })?;

                        // Verify user still exists
                        match UserQueries::find_by_id(state.database.pool(), user_id).await {
                            Ok(Some(user)) => {
                                return Ok(AuthenticatedUser {
                                    id: user.id,
                                    email: user.email,
                                    is_admin: user.is_admin,
                                });
                            }
                            Ok(None) => {
                                return Err((
                                    StatusCode::UNAUTHORIZED,
                                    Json(json!({"error": "User not found"})),
                                ).into_response());
                            }
                            Err(_) => {
                                return Err((
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({"error": "Database error"})),
                                ).into_response());
                            }
                        }
                    }
                    Err(_) => {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": "Invalid or expired token"})),
                        ).into_response());
                    }
                }
            }
        }

        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required"})),
        ).into_response())
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedApiKey {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Try Authorization header first
        let api_key = if let Some(auth_header) = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
        {
            match ApiKeyService::extract_key_from_header(auth_header) {
                Ok(key) => Some(key),
                Err(_) => None,
            }
        } else {
            // Try x-api-key header
            parts
                .headers
                .get("x-api-key")
                .and_then(|header| header.to_str().ok())
                .and_then(|key| ApiKeyService::validate_api_key_format(key).ok().map(|_| key.to_string()))
        };

        if let Some(api_key) = api_key {
            let key_hash = ApiKeyService::hash_api_key(&api_key);
            
            match ApiKeyQueries::find_by_key_hash(state.database.pool(), &key_hash).await {
                Ok(Some(db_key)) => {
                    // Check if API key has exceeded limits
                    match ApiKeyQueries::check_limits(state.database.pool(), db_key.id).await {
                        Ok((exceeded, limit_type, current, limit)) => {
                            if exceeded && limit_type != "none" {
                                return Err((
                                    StatusCode::TOO_MANY_REQUESTS,
                                    Json(json!({
                                        "error": "Rate limit exceeded",
                                        "limit_type": limit_type,
                                        "current_usage": current,
                                        "limit": limit
                                    })),
                                ).into_response());
                            }
                        }
                        Err(_) => {
                            return Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"error": "Failed to check limits"})),
                            ).into_response());
                        }
                    }

                    return Ok(AuthenticatedApiKey {
                        id: db_key.id,
                        owner_id: db_key.owner_id,
                        limits: db_key.limits_json,
                    });
                }
                Ok(None) => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": "Invalid API key"})),
                    ).into_response());
                }
                Err(_) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    ).into_response());
                }
            }
        }

        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "API key required"})),
        ).into_response())
    }
}
