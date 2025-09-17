use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Duration, Utc};
use crate::{
    handlers::AppState,
    error::{AppError, Result},
    utils::crypto,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiration time
    pub iat: usize,  // issued at
    pub iss: String, // issuer
}

#[derive(Clone)]
pub struct AuthLayer {
    secret: String,
}

impl AuthLayer {
    pub fn new() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string()),
        }
    }

    pub fn with_secret(secret: String) -> Self {
        Self { secret }
    }
}

// Extension trait to add user_id to request extensions
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub auth_method: AuthMethod,
}

#[derive(Debug)]
pub enum AuthMethod {
    JWT,
    ApiKey(String), // API key name
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let authenticated_user = match auth_header {
        Some(auth_value) => {
            if auth_value.starts_with("Bearer ") {
                // JWT authentication
                let token = &auth_value[7..];
                match verify_jwt_token(token, &state.config.jwt_secret).await {
                    Ok(user_id) => Some(AuthenticatedUser {
                        user_id,
                        auth_method: AuthMethod::JWT,
                    }),
                    Err(_) => None,
                }
            } else if auth_value.starts_with("ApiKey ") {
                // API Key authentication
                let api_key = &auth_value[7..];
                match verify_api_key(api_key, &state).await {
                    Ok((user_id, key_name)) => Some(AuthenticatedUser {
                        user_id,
                        auth_method: AuthMethod::ApiKey(key_name),
                    }),
                    Err(_) => None,
                }
            } else {
                None
            }
        }
        None => None,
    };

    match authenticated_user {
        Some(user) => {
            // Add user to request extensions
            request.extensions_mut().insert(user);
            Ok(next.run(request).await)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn verify_jwt_token(token: &str, secret: &str) -> Result<Uuid> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|_| AppError::Unauthorized)?;

    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Unauthorized)?;

    Ok(user_id)
}

async fn verify_api_key(api_key: &str, state: &AppState) -> Result<(Uuid, String)> {
    // Hash the provided API key
    let key_hash = crypto::hash_api_key(api_key)?;

    // Look up the API key in the database
    let stored_key = state.database.get_api_key_by_hash(&key_hash).await?
        .ok_or(AppError::Unauthorized)?;

    // Check if key is expired
    if let Some(expires_at) = stored_key.expires_at {
        if expires_at < Utc::now() {
            return Err(AppError::Unauthorized);
        }
    }

    // Update last used timestamp
    state.database.update_api_key_last_used(stored_key.id).await?;

    Ok((stored_key.user_id, stored_key.name))
}

pub fn generate_jwt_token(user_id: Uuid, secret: &str, expires_in_hours: i64) -> Result<String> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(expires_in_hours);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
        iss: "image-hosting-server".to_string(),
    };

    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let token = encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to generate JWT: {}", e)))?;

    Ok(token)
}

// Helper function to extract authenticated user from request
pub fn get_authenticated_user(request: &Request) -> Result<&AuthenticatedUser> {
    request
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or(AppError::Unauthorized)
}
