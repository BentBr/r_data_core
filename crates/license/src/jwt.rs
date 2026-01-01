#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::fs;

use crate::models::{LicenseClaims, LicenseType};

/// Create a license key JWT token
///
/// # Arguments
/// * `company` - Company name
/// * `license_type` - License type
/// * `private_key_path` - Path to private key file (RSA PEM format)
/// * `expires_days` - Optional expiration in days
///
/// # Errors
/// Returns an error if key file cannot be read or JWT encoding fails
pub fn create_license_key(
    company: &str,
    license_type: LicenseType,
    private_key_path: &str,
    expires_days: Option<u64>,
) -> Result<String, Error> {
    // Read private key
    let private_key = fs::read_to_string(private_key_path)
        .map_err(|e| Error::KeyFile(format!("Failed to read private key file: {e}")))?;

    // Generate license ID (UUID v7)
    let license_id = uuid::Uuid::now_v7().to_string();

    // Calculate expiration if provided
    let exp = expires_days.map(|days| {
        let now = time::OffsetDateTime::now_utc();
        // Safe to cast: days will never exceed i64::MAX
        #[allow(clippy::cast_possible_wrap)]
        let expires = now + time::Duration::days(days as i64);
        expires.unix_timestamp()
    });

    // Create claims
    let claims = LicenseClaims {
        version: "v1".to_string(),
        company: company.to_string(),
        license_type,
        issued_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| Error::TimeFormat(format!("Failed to format time: {e}")))?,
        license_id,
        exp,
    };

    // Encode JWT
    let encoding_key = EncodingKey::from_rsa_pem(private_key.as_bytes())
        .map_err(|e| Error::KeyFormat(format!("Invalid private key format: {e}")))?;

    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let token = encode(&header, &claims, &encoding_key)
        .map_err(|e| Error::JwtEncode(format!("Failed to encode JWT: {e}")))?;

    Ok(token)
}

/// Verify a license key JWT token
///
/// # Arguments
/// * `license_key` - JWT token string
/// * `public_key_path` - Path to public key file (RSA PEM format)
///
/// # Errors
/// Returns an error if key file cannot be read, JWT is invalid, or verification fails
pub fn verify_license_key(
    license_key: &str,
    public_key_path: &str,
) -> Result<LicenseClaims, Error> {
    // Read public key
    let public_key = fs::read_to_string(public_key_path)
        .map_err(|e| Error::KeyFile(format!("Failed to read public key file: {e}")))?;

    // Decode and verify JWT
    let decoding_key = DecodingKey::from_rsa_pem(public_key.as_bytes())
        .map_err(|e| Error::KeyFormat(format!("Invalid public key format: {e}")))?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.required_spec_claims.remove("exp");
    let token_data = decode::<LicenseClaims>(license_key, &decoding_key, &validation)
        .map_err(|e| Error::JwtDecode(format!("Failed to decode/verify JWT: {e}")))?;

    // Check expiration if present
    if let Some(exp) = token_data.claims.exp {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if now > exp {
            return Err(Error::Expired("License key has expired".to_string()));
        }
    }

    Ok(token_data.claims)
}

/// Error type for license operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Key file error: {0}")]
    KeyFile(String),
    #[error("Key format error: {0}")]
    KeyFormat(String),
    #[error("JWT encode error: {0}")]
    JwtEncode(String),
    #[error("JWT decode error: {0}")]
    JwtDecode(String),
    #[error("Time format error: {0}")]
    TimeFormat(String),
    #[error("License expired: {0}")]
    Expired(String),
}
