use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use crate::errors::{AppError, Result};

pub struct ApiKeyService;

impl ApiKeyService {
    pub fn generate_api_key() -> String {
        let key: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        format!("ik_{}", key) // Prefix with 'ik_' for image key
    }

    pub fn hash_api_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn validate_api_key_format(key: &str) -> Result<()> {
        if !key.starts_with("ik_") {
            return Err(AppError::Validation("API key must start with 'ik_'".to_string()));
        }

        if key.len() != 35 { // 'ik_' + 32 characters
            return Err(AppError::Validation("API key must be 35 characters long".to_string()));
        }

        let key_part = &key[3..];
        if !key_part.chars().all(|c| c.is_alphanumeric()) {
            return Err(AppError::Validation("API key contains invalid characters".to_string()));
        }

        Ok(())
    }

    pub fn extract_key_from_header(header_value: &str) -> Result<String> {
        if let Some(key) = header_value.strip_prefix("Bearer ") {
            Self::validate_api_key_format(key)?;
            Ok(key.to_string())
        } else {
            // Direct API key without Bearer prefix
            Self::validate_api_key_format(header_value)?;
            Ok(header_value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_generation() {
        let key = ApiKeyService::generate_api_key();
        assert!(key.starts_with("ik_"));
        assert_eq!(key.len(), 35);
        assert!(ApiKeyService::validate_api_key_format(&key).is_ok());
    }

    #[test]
    fn test_api_key_hashing() {
        let key = "ik_test123456789012345678901234567";
        let hash1 = ApiKeyService::hash_api_key(key);
        let hash2 = ApiKeyService::hash_api_key(key);
        
        assert_eq!(hash1, hash2); // Same input should produce same hash
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 character hex string
    }

    #[test]
    fn test_api_key_validation() {
        assert!(ApiKeyService::validate_api_key_format("ik_test123456789012345678901234567").is_ok());
        assert!(ApiKeyService::validate_api_key_format("invalid_key").is_err());
        assert!(ApiKeyService::validate_api_key_format("ik_short").is_err());
        assert!(ApiKeyService::validate_api_key_format("ik_test123456789012345678901234567!").is_err());
    }

    #[test]
    fn test_extract_key_from_header() {
        let key = "ik_test123456789012345678901234567";
        
        assert_eq!(
            ApiKeyService::extract_key_from_header(&format!("Bearer {}", key)).unwrap(),
            key
        );
        
        assert_eq!(
            ApiKeyService::extract_key_from_header(key).unwrap(),
            key
        );
    }
}
