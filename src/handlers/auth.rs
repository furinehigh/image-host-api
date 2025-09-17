use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    handlers::AppState,
    models::{User, ApiKey},
    error::{AppError, Result},
    utils::crypto,
    middleware::auth::{generate_jwt_token, AuthenticatedUser},
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String, // Only returned on creation
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse {
    pub id: Uuid,
    pub name: String,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)> {
    // Validate email format
    if !is_valid_email(&request.email) {
        return Err(AppError::Internal(anyhow::anyhow!("Invalid email format")));
    }

    // Validate password strength
    if request.password.len() < 8 {
        return Err(AppError::Internal(anyhow::anyhow!("Password must be at least 8 characters")));
    }

    // Check if user already exists
    if state.database.get_user_by_email(&request.email).await?.is_some() {
        return Err(AppError::Internal(anyhow::anyhow!("User already exists")));
    }

    // Hash password
    let password_hash = crypto::hash_password(&request.password)?;

    // Create user
    let user = state.database.create_user(&request.email, &password_hash).await?;

    // Generate JWT token
    let token = generate_jwt_token(user.id, &state.config.jwt_secret, 24)?; // 24 hours

    let response = AuthResponse {
        token,
        user_id: user.id,
        expires_in: 24 * 3600, // 24 hours in seconds
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    // Get user by email
    let user = state.database.get_user_by_email(&request.email).await?
        .ok_or(AppError::Unauthorized)?;

    // Verify password
    if !crypto::verify_password(&request.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    // Generate JWT token
    let token = generate_jwt_token(user.id, &state.config.jwt_secret, 24)?; // 24 hours

    let response = AuthResponse {
        token,
        user_id: user.user_id,
        expires_in: 24 * 3600, // 24 hours in seconds
    };

    Ok(Json(response))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>)> {
    // Generate API key
    let api_key = crypto::generate_api_key();
    let key_hash = crypto::hash_api_key(&api_key)?;

    // Calculate expiration date
    let expires_at = request.expires_in_days.map(|days| {
        chrono::Utc::now() + chrono::Duration::days(days)
    });

    // Create API key record
    let created_key = state.database.create_api_key(
        user.user_id,
        &key_hash,
        &request.name,
    ).await?;

    let response = ApiKeyResponse {
        id: created_key.id,
        name: created_key.name,
        key: api_key, // Only returned on creation
        created_at: created_key.created_at,
        expires_at: created_key.expires_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<ApiKeyListResponse>>> {
    // This would require a new database method to list user's API keys
    // For now, return empty list
    let keys = Vec::new(); // TODO: Implement database method

    let response: Vec<ApiKeyListResponse> = keys.into_iter().map(|key: ApiKey| {
        ApiKeyListResponse {
            id: key.id,
            name: key.name,
            last_used_at: key.last_used_at,
            created_at: key.created_at,
            expires_at: key.expires_at,
        }
    }).collect();

    Ok(Json(response))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> Result<StatusCode> {
    // This would require a new database method to delete API key
    // TODO: Implement database method to delete API key by ID and user_id
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh_token(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<AuthResponse>> {
    // Generate new JWT token
    let token = generate_jwt_token(user.user_id, &state.config.jwt_secret, 24)?; // 24 hours

    let response = AuthResponse {
        token,
        user_id: user.user_id,
        expires_in: 24 * 3600, // 24 hours in seconds
    };

    Ok(Json(response))
}

fn is_valid_email(email: &str) -> bool {
    // Simple email validation
    email.contains('@') && email.contains('.') && email.len() > 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::database::Database;
    use crate::services::redis::RedisService;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn create_test_state() -> AppState {
        // This would need to be adapted based on your actual Database and RedisService constructors
        AppState {
            database: Database::new_test().expect("Failed to create test database"),
            redis: RedisService::new_test().expect("Failed to create test Redis service"),
            config: Config::test_config(),
        }
    }

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name+tag@domain.co.uk"));
        assert!(!is_valid_email("invalid-email"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email(""));
    }

    #[tokio::test]
    async fn test_register_valid_user() {
        let state = create_test_state();
        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };

        let result = register(State(state), Json(request)).await;
        assert!(result.is_ok());
        
        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(!response.token.is_empty());
        assert_eq!(response.expires_in, 24 * 3600);
    }

    #[tokio::test]
    async fn test_register_invalid_email() {
        let state = create_test_state();
        let request = RegisterRequest {
            email: "invalid-email".to_string(),
            password: "securepassword123".to_string(),
        };

        let result = register(State(state), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_weak_password() {
        let state = create_test_state();
        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "weak".to_string(),
        };

        let result = register(State(state), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_login_valid_credentials() {
        let state = create_test_state();
        
        // First register a user
        let register_request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };
        let _ = register(State(state.clone()), Json(register_request)).await.unwrap();

        // Then try to login
        let login_request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
        };

        let result = login(State(state), Json(login_request)).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.token.is_empty());
        assert_eq!(response.expires_in, 24 * 3600);
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let state = create_test_state();
        let request = LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "wrongpassword".to_string(),
        };

        let result = login(State(state), Json(request)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Unauthorized));
    }
}
