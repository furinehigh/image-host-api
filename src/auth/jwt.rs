use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::errors::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub email: String,
    pub is_admin: bool,
    pub exp: i64,
    pub iat: i64,
    pub token_type: TokenType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_duration: Duration,
    refresh_token_duration: Duration,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            access_token_duration: Duration::hours(1),
            refresh_token_duration: Duration::days(7),
        }
    }

    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        is_admin: bool,
    ) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            is_admin,
            exp: (now + self.access_token_duration).timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Access,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Auth(format!("Failed to generate access token: {}", e)))
    }

    pub fn generate_refresh_token(
        &self,
        user_id: Uuid,
        email: &str,
        is_admin: bool,
    ) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            is_admin,
            exp: (now + self.refresh_token_duration).timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Refresh,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Auth(format!("Failed to generate refresh token: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| AppError::Auth(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    pub fn verify_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;
        
        match claims.token_type {
            TokenType::Access => Ok(claims),
            TokenType::Refresh => Err(AppError::Auth("Expected access token, got refresh token".to_string())),
        }
    }

    pub fn verify_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;
        
        match claims.token_type {
            TokenType::Refresh => Ok(claims),
            TokenType::Access => Err(AppError::Auth("Expected refresh token, got access token".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_verification() {
        let jwt_service = JwtService::new("test-secret");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let access_token = jwt_service.generate_access_token(user_id, email, false).unwrap();
        let refresh_token = jwt_service.generate_refresh_token(user_id, email, false).unwrap();

        let access_claims = jwt_service.verify_access_token(&access_token).unwrap();
        let refresh_claims = jwt_service.verify_refresh_token(&refresh_token).unwrap();

        assert_eq!(access_claims.sub, user_id.to_string());
        assert_eq!(access_claims.email, email);
        assert_eq!(refresh_claims.sub, user_id.to_string());
        assert_eq!(refresh_claims.email, email);
    }
}
