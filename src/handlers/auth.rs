use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{JwtService, PasswordService},
    database::queries::UserQueries,
    errors::{AppError, Result},
    handlers::AppState,
    models::{AuthResponse, CreateUserRequest, LoginRequest, UserResponse},
};

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>> {
    // Validate email format
    if !request.email.contains('@') {
        return Err(AppError::Validation("Invalid email format".to_string()));
    }

    // Validate password strength
    PasswordService::validate_password_strength(&request.password)?;

    // Check if user already exists
    if let Some(_) = UserQueries::find_by_email(state.database.pool(), &request.email).await? {
        return Err(AppError::Validation("User with this email already exists".to_string()));
    }

    // Hash password
    let password_hash = PasswordService::hash_password(&request.password)?;

    // Create user
    let user = UserQueries::create_user(
        state.database.pool(),
        &request.email,
        &password_hash,
    ).await?;

    // Generate JWT tokens
    let jwt_service = JwtService::new(&state.config.jwt_secret);
    let access_token = jwt_service.generate_access_token(user.id, &user.email, user.is_admin)?;
    let refresh_token = jwt_service.generate_refresh_token(user.id, &user.email, user.is_admin)?;

    let response = AuthResponse {
        access_token,
        refresh_token,
        user: UserResponse::from(user),
    };

    Ok(Json(json!({
        "message": "User registered successfully",
        "data": response
    })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>> {
    // Find user by email
    let user = UserQueries::find_by_email(state.database.pool(), &request.email)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid email or password".to_string()))?;

    // Verify password
    if !PasswordService::verify_password(&request.password, &user.password_hash)? {
        return Err(AppError::Auth("Invalid email or password".to_string()));
    }

    // Generate JWT tokens
    let jwt_service = JwtService::new(&state.config.jwt_secret);
    let access_token = jwt_service.generate_access_token(user.id, &user.email, user.is_admin)?;
    let refresh_token = jwt_service.generate_refresh_token(user.id, &user.email, user.is_admin)?;

    let response = AuthResponse {
        access_token,
        refresh_token,
        user: UserResponse::from(user),
    };

    Ok(Json(json!({
        "message": "Login successful",
        "data": response
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    let refresh_token = request
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("Refresh token is required".to_string()))?;

    // Verify refresh token
    let jwt_service = JwtService::new(&state.config.jwt_secret);
    let claims = jwt_service.verify_refresh_token(refresh_token)?;

    // Parse user ID
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Auth("Invalid user ID in token".to_string()))?;

    // Verify user still exists
    let user = UserQueries::find_by_id(state.database.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;

    // Generate new access token
    let access_token = jwt_service.generate_access_token(user.id, &user.email, user.is_admin)?;

    Ok(Json(json!({
        "message": "Token refreshed successfully",
        "data": {
            "access_token": access_token,
            "user": UserResponse::from(user)
        }
    })))
}

pub async fn logout(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    // In a stateless JWT system, logout is handled client-side by discarding tokens
    // For enhanced security, you could maintain a token blacklist in Redis
    Ok(Json(json!({
        "message": "Logged out successfully"
    })))
}
