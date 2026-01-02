#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::fs;
use std::path::Path;

use crate::api::call_verification_api;
use crate::jwt::{create_license_key, verify_license_key};
use crate::models::{LicenseClaims, LicenseType};

/// Result of license creation
#[derive(Debug)]
pub struct LicenseCreationResult {
    /// The created license key token
    pub token: String,
    /// Company name
    pub company: String,
    /// License type
    pub license_type: String,
    /// License ID
    pub license_id: String,
    /// Issue date
    pub issued_at: time::OffsetDateTime,
    /// Version
    pub version: String,
    /// Expiration date (if any)
    pub expires: Option<time::OffsetDateTime>,
}

/// License check state (matches `LicenseState` from service)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseCheckState {
    /// No license key provided
    None,
    /// License key is invalid (API returned valid=false)
    Invalid,
    /// Network/technical error during verification
    Error,
    /// License key is valid
    Valid,
}

/// Result of license API check (matches `LicenseVerificationResult` format from service)
#[derive(Debug, Clone)]
pub struct LicenseCheckResult {
    /// License state
    pub state: LicenseCheckState,
    /// Company name (if license is present)
    pub company: Option<String>,
    /// License type (if license is present)
    pub license_type: Option<String>,
    /// License ID (if license is present)
    pub license_id: Option<String>,
    /// Issue date (if license is present)
    pub issued_at: Option<time::OffsetDateTime>,
    /// License version
    pub version: Option<String>,
    /// Verification timestamp
    pub verified_at: time::OffsetDateTime,
    /// Error message (only present if state is "error" or "invalid")
    pub error_message: Option<String>,
}

/// Result of license verification
#[derive(Debug)]
pub struct LicenseVerificationDisplayResult {
    /// Whether the license is valid
    pub is_valid: bool,
    /// Company name
    pub company: String,
    /// License type
    pub license_type: String,
    /// License ID
    pub license_id: String,
    /// Issue date
    pub issued_at: time::OffsetDateTime,
    /// Version
    pub version: String,
    /// Expiration date (if any)
    pub expires: Option<time::OffsetDateTime>,
    /// Error message (if invalid)
    pub error: Option<String>,
}

/// Service for license tool operations
pub struct LicenseToolService;

impl LicenseToolService {
    /// Create a new license key
    ///
    /// # Arguments
    /// * `company` - Company name
    /// * `license_type` - License type string
    /// * `private_key_file` - Path to private key file
    /// * `expires_days` - Optional expiration in days
    ///
    /// # Errors
    /// Returns an error if license creation fails
    pub fn create_license(
        company: &str,
        license_type: &str,
        private_key_file: &str,
        expires_days: Option<u64>,
    ) -> Result<LicenseCreationResult, Box<dyn std::error::Error + Send + Sync>> {
        // Parse license type
        let license_type_enum = license_type.parse::<LicenseType>().map_err(|e| {
            format!(
                "Invalid license type: {e}. Available types: {}",
                LicenseType::all_variants().join(", ")
            )
        })?;

        // Create the license key
        let token = create_license_key(company, license_type_enum, private_key_file, expires_days)
            .map_err(|e| format!("Failed to create license key: {e}"))?;

        // Try to verify to get claims for display
        // Note: We try to construct public key path, but if it fails, we decode the JWT directly
        let public_key_file = private_key_file.replace("private", "public");
        let claims = verify_license_key(&token, &public_key_file).or_else(|_| {
            // Fallback: decode JWT without verification to get claims
            Self::decode_license_claims(&token)
        })?;

        // Parse issued_at and expiration
        let issued_at = time::OffsetDateTime::parse(
            &claims.issued_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| format!("Failed to parse issued_at: {e}"))?;
        let expires = claims
            .exp
            .and_then(|exp| time::OffsetDateTime::from_unix_timestamp(exp).ok());

        Ok(LicenseCreationResult {
            token,
            company: claims.company,
            license_type: claims.license_type.to_string(),
            license_id: claims.license_id,
            issued_at,
            version: claims.version,
            expires,
        })
    }

    /// Verify a license key
    ///
    /// # Arguments
    /// * `license_key` - License key JWT token
    /// * `public_key_file` - Path to public key file
    ///
    /// # Errors
    /// Returns an error if verification fails
    pub fn verify_license(
        license_key: &str,
        public_key_file: &str,
    ) -> Result<LicenseVerificationDisplayResult, Box<dyn std::error::Error + Send + Sync>> {
        match verify_license_key(license_key, public_key_file) {
            Ok(claims) => {
                // Parse issued_at and expiration
                let issued_at = time::OffsetDateTime::parse(
                    &claims.issued_at,
                    &time::format_description::well_known::Rfc3339,
                )
                .map_err(|e| format!("Failed to parse issued_at: {e}"))?;
                let expires = claims
                    .exp
                    .and_then(|exp| time::OffsetDateTime::from_unix_timestamp(exp).ok());

                Ok(LicenseVerificationDisplayResult {
                    is_valid: true,
                    company: claims.company,
                    license_type: claims.license_type.to_string(),
                    license_id: claims.license_id,
                    issued_at,
                    version: claims.version,
                    expires,
                    error: None,
                })
            }
            Err(e) => Ok(LicenseVerificationDisplayResult {
                is_valid: false,
                company: String::new(),
                license_type: String::new(),
                license_id: String::new(),
                issued_at: time::OffsetDateTime::now_utc(),
                version: String::new(),
                expires: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Write license key to file
    ///
    /// # Arguments
    /// * `token` - License key token
    /// * `output_path` - Path to output file
    ///
    /// # Errors
    /// Returns an error if file write fails
    pub fn write_license_to_file(
        token: &str,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        fs::write(output_path, token)?;
        Ok(())
    }

    /// Check license key against the verification API using the same logic as the service
    ///
    /// This function uses the same API call logic as the maintenance task's `LicenseService`.
    /// It returns a result that matches the service's `LicenseVerificationResult` format.
    ///
    /// # Arguments
    /// * `license_key` - License key JWT token
    /// * `verification_url` - URL of the license verification API (from config)
    ///
    /// # Errors
    /// Returns an error if the API call fails
    pub async fn check_license(
        license_key: &str,
        verification_url: &str,
    ) -> Result<LicenseCheckResult, Box<dyn std::error::Error + Send + Sync>> {
        // Call the same API function that the service uses
        let api_result = call_verification_api(license_key, verification_url).await?;

        // Parse issued_at (same as service does)
        let issued_at = time::OffsetDateTime::parse(
            &api_result.claims.issued_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| format!("Failed to parse issued_at: {e}"))?;

        Ok(LicenseCheckResult {
            state: if api_result.is_valid {
                LicenseCheckState::Valid
            } else {
                LicenseCheckState::Invalid
            },
            company: Some(api_result.claims.company),
            license_type: Some(api_result.claims.license_type.to_string()),
            license_id: Some(api_result.claims.license_id),
            issued_at: Some(issued_at),
            version: Some(api_result.claims.version),
            verified_at: time::OffsetDateTime::now_utc(),
            error_message: api_result.error_message,
        })
    }

    /// Decode license claims from JWT without verification (for display purposes)
    fn decode_license_claims(
        license_key: &str,
    ) -> Result<LicenseClaims, Box<dyn std::error::Error + Send + Sync>> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let parts: Vec<&str> = license_key.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }

        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload)?;
        let claims: LicenseClaims = serde_json::from_slice(&decoded)?;

        Ok(claims)
    }
}
