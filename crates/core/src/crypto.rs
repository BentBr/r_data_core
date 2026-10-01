use crate::error::{Error, Result};
use std::sync::OnceLock;

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

/// A real Argon2id hash of a random secret, generated once per process.
///
/// Used purely to burn the same amount of CPU as a genuine verification.
static DUMMY_HASH: OnceLock<String> = OnceLock::new();

/// Perform a throw-away verification so an unknown account costs the same
/// wall-clock time as a known one.
///
/// Without this the "no such user" path returns before any Argon2 work and the
/// timing difference becomes a username-enumeration oracle.
pub fn verify_dummy_password(password: &str) {
    let hash = DUMMY_HASH.get_or_init(|| {
        let secret = SaltString::generate(&mut OsRng).to_string();
        hash_password_argon2(&secret).unwrap_or_default()
    });
    let _ = verify_password_argon2(password, hash);
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
    fn dummy_verification_uses_a_real_hash() {
        // A parseable hash is what makes the work — and therefore the timing —
        // comparable to a genuine verification.
        verify_dummy_password("anything");
        let hash = DUMMY_HASH.get().map(String::as_str).unwrap_or_default();
        assert!(hash.starts_with("$argon2id$"));
        assert!(!verify_password_argon2("anything", hash));
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

/// Compare two secrets without leaking anything through timing.
///
/// A plain `==` on `str` or `String` short-circuits at the first differing
/// byte, so the time it takes reveals how long a shared prefix the attacker
/// guessed. Repeated over an alphabet that is a byte-at-a-time recovery of the
/// secret (CWE-208).
///
/// Both sides are hashed first, then the two fixed-size digests are compared
/// with [`subtle::ConstantTimeEq`]. Hashing is what makes the *length* safe
/// too: comparing the raw bytes would need a loop bounded by one of the two
/// lengths, which leaks the secret's length under an input-length sweep. SHA-256
/// is a fixed 32 bytes whatever goes in, so neither the content nor the length
/// of `expected` is observable.
///
/// The digest is not a security boundary here — it is only a length
/// equaliser — so a plain hash without a salt is the right primitive. This is
/// for comparing secrets that are already high-entropy tokens; use
/// [`verify_password_argon2`] for user-chosen passwords.
#[must_use]
pub fn constant_time_eq(provided: &str, expected: &str) -> bool {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let provided = Sha256::digest(provided.as_bytes());
    let expected = Sha256::digest(expected.as_bytes());

    provided.ct_eq(&expected).into()
}

#[cfg(test)]
mod constant_time_eq_tests {
    use super::constant_time_eq;

    #[test]
    fn identical_secrets_match() {
        assert!(constant_time_eq("s3cret-token", "s3cret-token"));
    }

    #[test]
    fn a_different_secret_does_not_match() {
        assert!(!constant_time_eq("s3cret-token", "s3cret-tokeN"));
    }

    #[test]
    fn a_shared_prefix_does_not_match() {
        assert!(
            !constant_time_eq("s3cret", "s3cret-token"),
            "a prefix of the secret must be rejected like any other wrong value"
        );
    }

    #[test]
    fn a_longer_guess_does_not_match() {
        assert!(!constant_time_eq("s3cret-token-and-more", "s3cret-token"));
    }

    #[test]
    fn empty_strings_match_each_other() {
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn an_empty_guess_does_not_match_a_real_secret() {
        assert!(!constant_time_eq("", "s3cret-token"));
    }

    /// Non-ASCII must not panic or be truncated: the comparison is over bytes.
    #[test]
    fn multibyte_secrets_are_compared_by_bytes() {
        assert!(constant_time_eq("schlüssel-🔑", "schlüssel-🔑"));
        assert!(!constant_time_eq("schlüssel-🔑", "schlussel-🔑"));
    }
}
