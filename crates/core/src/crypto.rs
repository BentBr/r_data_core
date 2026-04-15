use crate::error::{Error, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

/// Hash a password using Argon2id with standardized parameters.
///
/// Parameters: m=19456 (19 MB), t=2 iterations, p=1 parallelism.
///
/// # Errors
/// Returns `Error::PasswordHash` if hashing fails.
pub fn hash_password_argon2(password: &str) -> Result<String> {
    let params = Params::new(19456, 2, 1, None)
        .map_err(|e| Error::PasswordHash(format!("Invalid parameters: {e}")))?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::PasswordHash(format!("Failed to hash password: {e}")))
        .map(|hash| hash.to_string())
}

/// Verify a password against an Argon2 hash.
#[must_use]
pub fn verify_password_argon2(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_round_trip() {
        let password = "test_password_123";
        let hash = hash_password_argon2(password).unwrap();

        assert!(verify_password_argon2(password, &hash));
    }

    #[test]
    fn test_wrong_password_fails() {
        let hash = hash_password_argon2("correct_password").unwrap();

        assert!(!verify_password_argon2("wrong_password", &hash));
    }

    #[test]
    fn test_invalid_hash_fails() {
        assert!(!verify_password_argon2("password", "not_a_valid_hash"));
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let hash1 = hash_password_argon2("same_password").unwrap();
        let hash2 = hash_password_argon2("same_password").unwrap();

        // Different salts produce different hashes
        assert_ne!(hash1, hash2);
        // Both verify correctly
        assert!(verify_password_argon2("same_password", &hash1));
        assert!(verify_password_argon2("same_password", &hash2));
    }
}
