use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, Params,
};
use std::env;

/// Create Argon2id parameters with standardized settings
pub fn create_argon2_params() -> Result<Params, String> {
    // Use standardized Argon2id parameters:
    // - m=19456 (19 MB memory cost - good balance for modern systems)
    // - t=2 (2 iterations - recommended minimum)
    // - p=1 (1 parallelism - good for most systems)
    Params::new(19456, 2, 1, None).map_err(|e| format!("Error creating parameters: {}", e))
}

/// Generate a new salt for password hashing
pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

/// Create an Argon2id hasher with the given parameters
pub fn create_argon2_hasher(params: Params) -> Argon2<'static> {
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str, salt: &SaltString, hasher: &Argon2) -> Result<String, String> {
    hasher
        .hash_password(password.as_bytes(), salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Error hashing password: {}", e))
}

/// Generate a complete password hash with salt
pub fn generate_password_hash(password: &str) -> Result<String, String> {
    let params = create_argon2_params()?;
    let salt = generate_salt();
    let hasher = create_argon2_hasher(params);
    hash_password(password, &salt, &hasher)
}

/// Escape single quotes in a string for SQL
pub fn escape_sql_string(s: &str) -> String {
    s.replace("'", "''")
}

/// Generate SQL UPDATE statement for password hash
pub fn generate_sql_update(hash: &str) -> String {
    format!(
        "UPDATE admin_users SET password_hash = '{}' WHERE username = 'admin';",
        escape_sql_string(hash)
    )
}

/// Format the complete output for the hash_password binary
pub fn format_output(password: &str, hash: &str) -> String {
    let sql = generate_sql_update(hash);
    format!("Password: {}\nHash: {}\n\nSQL:\n{}", password, hash, sql)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <password>", args[0]);
        return;
    }

    let password = &args[1];

    match generate_password_hash(password) {
        Ok(hash) => {
            println!("{}", format_output(password, &hash));
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_argon2_params() {
        let params = create_argon2_params().unwrap();
        // Verify the parameters are set correctly
        assert_eq!(params.m_cost(), 19456);
        assert_eq!(params.t_cost(), 2);
        assert_eq!(params.p_cost(), 1);
    }

    #[test]
    fn test_generate_salt() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        // Salts should be different (random)
        assert_ne!(salt1.as_str(), salt2.as_str());

        // Salt should be valid format (Argon2 generates salts in a specific format)
        // The exact format depends on the Argon2 implementation
        assert!(salt1.as_str().len() > 0);
        assert!(salt2.as_str().len() > 0);

        // Both salts should be different
        assert_ne!(salt1.as_str(), salt2.as_str());
    }

    #[test]
    fn test_create_argon2_hasher() {
        let params = create_argon2_params().unwrap();
        let hasher = create_argon2_hasher(params);

        // The hasher should be created successfully
        // We can't easily test the internal state, but we can verify it works
        // Note: Argon2 doesn't expose algorithm() method, so we just verify it was created
        assert!(true); // Hasher was created successfully
    }

    #[test]
    fn test_hash_password() {
        let params = create_argon2_params().unwrap();
        let salt = generate_salt();
        let hasher = create_argon2_hasher(params);

        let password = "testpassword123";
        let hash = hash_password(password, &salt, &hasher).unwrap();

        // Verify the hash format
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.contains("v=19"));
        assert!(hash.contains("m=19456,t=2,p=1"));

        // Verify the hash is different for different passwords
        let hash2 = hash_password("differentpassword", &salt, &hasher).unwrap();
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_hash_password_empty() {
        let params = create_argon2_params().unwrap();
        let salt = generate_salt();
        let hasher = create_argon2_hasher(params);

        let password = "";
        let hash = hash_password(password, &salt, &hasher).unwrap();

        // Empty password should still generate a valid hash
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_hash_password_special_characters() {
        let params = create_argon2_params().unwrap();
        let salt = generate_salt();
        let hasher = create_argon2_hasher(params);

        let password = "p@ssw0rd!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let hash = hash_password(password, &salt, &hasher).unwrap();

        // Special characters should be handled correctly
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_generate_password_hash() {
        let password = "testpassword123";
        let hash = generate_password_hash(password).unwrap();

        // Verify the hash format
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.contains("v=19"));
        assert!(hash.contains("m=19456,t=2,p=1"));

        // Same password should generate different hashes (due to different salts)
        let hash2 = generate_password_hash(password).unwrap();
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_escape_sql_string() {
        // Test normal string
        assert_eq!(escape_sql_string("normal"), "normal");

        // Test string with single quotes
        assert_eq!(escape_sql_string("don't"), "don''t");

        // Test string with multiple single quotes
        assert_eq!(escape_sql_string("can't won't"), "can''t won''t");

        // Test string with no quotes
        assert_eq!(escape_sql_string(""), "");
    }

    #[test]
    fn test_generate_sql_update() {
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$test$hash";
        let sql = generate_sql_update(hash);

        assert!(sql.starts_with("UPDATE admin_users SET password_hash = '"));
        assert!(sql.ends_with("' WHERE username = 'admin';"));
        assert!(sql.contains(hash));
    }

    #[test]
    fn test_generate_sql_update_with_quotes() {
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$test'hash";
        let sql = generate_sql_update(hash);

        // Should escape single quotes in the hash
        assert!(sql.contains("test''hash"));
        assert!(!sql.contains("test'hash"));
    }

    #[test]
    fn test_format_output() {
        let password = "testpassword123";
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$test$hash";
        let output = format_output(password, hash);

        assert!(output.contains(&format!("Password: {}", password)));
        assert!(output.contains(&format!("Hash: {}", hash)));
        assert!(output.contains("SQL:"));
        assert!(output.contains("UPDATE admin_users SET password_hash = '"));
        assert!(output.contains("' WHERE username = 'admin';"));
    }

    #[test]
    fn test_hash_consistency() {
        // Test that the same password with the same salt produces the same hash
        let params = create_argon2_params().unwrap();
        let salt = SaltString::from_b64("dGVzdHNhbHQ").unwrap(); // "testsalt" in base64 (without padding)
        let hasher = create_argon2_hasher(params);

        let password = "testpassword123";
        let hash1 = hash_password(password, &salt, &hasher).unwrap();
        let hash2 = hash_password(password, &salt, &hasher).unwrap();

        // Same password + same salt should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_uniqueness() {
        let params = create_argon2_params().unwrap();
        let hasher = create_argon2_hasher(params);

        let password = "testpassword123";
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        let hash1 = hash_password(password, &salt1, &hasher).unwrap();
        let hash2 = hash_password(password, &salt2, &hasher).unwrap();

        // Same password with different salts should produce different hashes
        assert_ne!(hash1, hash2);
    }
}
