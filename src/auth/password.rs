use bcrypt::{hash, verify, DEFAULT_COST};
use crate::errors::{AppError, Result};

pub struct PasswordService;

impl PasswordService {
    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST)
            .map_err(|e| AppError::Auth(format!("Failed to hash password: {}", e)))
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash)
            .map_err(|e| AppError::Auth(format!("Failed to verify password: {}", e)))
    }

    pub fn validate_password_strength(password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(AppError::Validation("Password must be at least 8 characters long".to_string()));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        if !has_uppercase {
            return Err(AppError::Validation("Password must contain at least one uppercase letter".to_string()));
        }

        if !has_lowercase {
            return Err(AppError::Validation("Password must contain at least one lowercase letter".to_string()));
        }

        if !has_digit {
            return Err(AppError::Validation("Password must contain at least one digit".to_string()));
        }

        if !has_special {
            return Err(AppError::Validation("Password must contain at least one special character".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "TestPassword123!";
        let hash = PasswordService::hash_password(password).unwrap();
        
        assert!(PasswordService::verify_password(password, &hash).unwrap());
        assert!(!PasswordService::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_strength_validation() {
        assert!(PasswordService::validate_password_strength("TestPassword123!").is_ok());
        assert!(PasswordService::validate_password_strength("weak").is_err());
        assert!(PasswordService::validate_password_strength("NoDigits!").is_err());
        assert!(PasswordService::validate_password_strength("nouppercaseorspecial123").is_err());
    }
}
