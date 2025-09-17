use sha2::{Sha256, Digest};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::{thread_rng, Rng};
use anyhow::Result;

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let is_valid = verify(password, hash)?;
    Ok(is_valid)
}

pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn generate_api_key() -> String {
    let mut rng = thread_rng();
    let key: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            match idx {
                0..=25 => (b'a' + idx) as char,
                26..=51 => (b'A' + (idx - 26)) as char,
                _ => (b'0' + (idx - 52)) as char,
            }
        })
        .collect();
    
    format!("img_{}", key)
}

pub fn hash_api_key(api_key: &str) -> Result<String> {
    let hashed = hash(api_key, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_api_key(api_key: &str, hash: &str) -> Result<bool> {
    let is_valid = verify(api_key, hash)?;
    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "test_password_123";
        
        // Hash password
        let hash = hash_password(password).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password); // Hash should be different from original
        
        // Verify correct password
        let is_valid = verify_password(password, &hash).unwrap();
        assert!(is_valid);
        
        // Verify incorrect password
        let is_invalid = verify_password("wrong_password", &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_password_hash_uniqueness() {
        let password = "same_password";
        
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        
        // Same password should produce different hashes due to salt
        assert_ne!(hash1, hash2);
        
        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_sha256_calculation() {
        let data = b"Hello, World!";
        let hash = calculate_sha256(data);
        
        // SHA256 of "Hello, World!" should be consistent
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(hash, expected);
        
        // Empty data
        let empty_hash = calculate_sha256(b"");
        let expected_empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(empty_hash, expected_empty);
    }

    #[test]
    fn test_api_key_generation() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        
        // Keys should be different
        assert_ne!(key1, key2);
        
        // Keys should have correct format
        assert!(key1.starts_with("img_"));
        assert!(key2.starts_with("img_"));
        
        // Keys should have correct length (img_ + 32 chars)
        assert_eq!(key1.len(), 36);
        assert_eq!(key2.len(), 36);
        
        // Keys should only contain alphanumeric characters after prefix
        let key_part = &key1[4..];
        assert!(key_part.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_api_key_hashing_and_verification() {
        let api_key = generate_api_key();
        
        // Hash API key
        let hash = hash_api_key(&api_key).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, api_key);
        
        // Verify correct API key
        let is_valid = verify_api_key(&api_key, &hash).unwrap();
        assert!(is_valid);
        
        // Verify incorrect API key
        let wrong_key = generate_api_key();
        let is_invalid = verify_api_key(&wrong_key, &hash).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_api_key_hash_uniqueness() {
        let api_key = generate_api_key();
        
        let hash1 = hash_api_key(&api_key).unwrap();
        let hash2 = hash_api_key(&api_key).unwrap();
        
        // Same API key should produce different hashes due to salt
        assert_ne!(hash1, hash2);
        
        // But both should verify correctly
        assert!(verify_api_key(&api_key, &hash1).unwrap());
        assert!(verify_api_key(&api_key, &hash2).unwrap());
    }

    #[test]
    fn test_sha256_consistency() {
        let data = b"test data for consistency";
        
        // Multiple calls should produce same hash
        let hash1 = calculate_sha256(data);
        let hash2 = calculate_sha256(data);
        let hash3 = calculate_sha256(data);
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_empty_password_handling() {
        let empty_password = "";
        let hash = hash_password(empty_password).unwrap();
        
        // Should be able to hash empty password
        assert!(!hash.is_empty());
        
        // Should verify correctly
        let is_valid = verify_password(empty_password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_long_password_handling() {
        let long_password = "a".repeat(1000);
        let hash = hash_password(&long_password).unwrap();
        
        // Should be able to hash long password
        assert!(!hash.is_empty());
        
        // Should verify correctly
        let is_valid = verify_password(&long_password, &hash).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_unicode_password_handling() {
        let unicode_password = "пароль123🔒";
        let hash = hash_password(unicode_password).unwrap();
        
        // Should be able to hash unicode password
        assert!(!hash.is_empty());
        
        // Should verify correctly
        let is_valid = verify_password(unicode_password, &hash).unwrap();
        assert!(is_valid);
    }
}
