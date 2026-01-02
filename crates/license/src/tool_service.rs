#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use r_data_core_core::cache::CacheManager;

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
    /// Expiration date (if license is present and has expiration)
    pub expires_at: Option<time::OffsetDateTime>,
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
    /// * `expires_at` - Optional expiration date (if None, license never expires)
    ///
    /// # Errors
    /// Returns an error if license creation fails
    pub fn create_license(
        company: &str,
        license_type: &str,
        private_key_file: &str,
        expires_at: Option<time::OffsetDateTime>,
    ) -> Result<LicenseCreationResult, Box<dyn std::error::Error + Send + Sync>> {
        // Parse license type
        let license_type_enum = license_type.parse::<LicenseType>().map_err(|e| {
            format!(
                "Invalid license type: {e}. Available types: {}",
                LicenseType::all_variants().join(", ")
            )
        })?;

        // Create the license key
        let token = create_license_key(company, license_type_enum, private_key_file, expires_at)
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
                error: Some(format!("{e}")),
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
    /// This function uses the same API call and cache update logic as `LicenseService::verify_license()`.
    /// It clears the cache entry first to force a fresh verification, then calls the API and updates cache.
    ///
    /// # Arguments
    /// * `config` - License configuration
    /// * `cache_manager` - Cache manager (required, same as maintenance worker)
    ///
    /// # Errors
    /// Returns an error if the API call fails
    pub async fn check_license(
        config: r_data_core_core::config::LicenseConfig,
        cache_manager: Arc<CacheManager>,
    ) -> Result<LicenseCheckResult, Box<dyn std::error::Error + Send + Sync>> {
        // Get license key or return None state
        let license_key = match &config.license_key {
            Some(key) if !key.is_empty() => key,
            _ => return Ok(Self::none_state_result()),
        };

        // Clear cache before verification (forces fresh check)
        Self::clear_cache_for_license(license_key, &cache_manager).await;

        // Call API and handle errors
        match call_verification_api(license_key, &config.verification_url).await {
            Ok(api_result) => Self::handle_successful_verification(api_result, cache_manager).await,
            Err(e) => Self::handle_network_error(license_key, e, cache_manager).await,
        }
    }

    /// Return a None state result (no license key provided)
    fn none_state_result() -> LicenseCheckResult {
        LicenseCheckResult {
            state: LicenseCheckState::None,
            company: None,
            license_type: None,
            license_id: None,
            issued_at: None,
            expires_at: None,
            version: None,
            verified_at: time::OffsetDateTime::now_utc(),
            error_message: None,
        }
    }

    /// Clear cache entry for a license key
    async fn clear_cache_for_license(license_key: &str, cache_manager: &Arc<CacheManager>) {
        if let Ok(license_id) = Self::extract_license_id(license_key) {
            let cache_key = format!("license:verification:{license_id}");
            let _ = cache_manager.delete(&cache_key).await;
        }
    }

    /// Handle network error during license verification
    async fn handle_network_error(
        license_key: &str,
        error: Box<dyn std::error::Error + Send + Sync>,
        cache_manager: Arc<CacheManager>,
    ) -> Result<LicenseCheckResult, Box<dyn std::error::Error + Send + Sync>> {
        use crate::api::decode_license_claims;

        // Try to decode claims for error reporting
        let claims = decode_license_claims(license_key).ok();
        let company = claims.as_ref().map(|c| c.company.clone());
        let license_id = claims.as_ref().map(|c| c.license_id.clone());
        let license_type = claims.as_ref().map(|c| c.license_type.to_string());
        let version = claims.as_ref().map(|c| c.version.clone());
        let issued_at = claims.as_ref().and_then(|c| {
            time::OffsetDateTime::parse(
                &c.issued_at,
                &time::format_description::well_known::Rfc3339,
            )
            .ok()
        });
        let expires_at = claims
            .as_ref()
            .and_then(|c| c.exp)
            .and_then(|exp| time::OffsetDateTime::from_unix_timestamp(exp).ok());

        let verified_at = time::OffsetDateTime::now_utc();
        let error_message = Some(format!("Network error: {error}"));

        let result = LicenseCheckResult {
            state: LicenseCheckState::Error,
            company,
            license_type,
            license_id: license_id.clone(),
            issued_at,
            expires_at,
            version,
            verified_at,
            error_message,
        };

        // Cache the error result with NEW timestamp (invalidates old cache)
        if license_id.is_some() {
            Self::cache_verification_result(&result, &cache_manager).await;
        }

        Ok(result)
    }

    /// Handle successful API verification
    async fn handle_successful_verification(
        api_result: crate::api::LicenseVerificationApiResult,
        cache_manager: Arc<CacheManager>,
    ) -> Result<LicenseCheckResult, Box<dyn std::error::Error + Send + Sync>> {
        // Parse issued_at (same as service does)
        let issued_at = time::OffsetDateTime::parse(
            &api_result.claims.issued_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|e| format!("Failed to parse issued_at: {e}"))?;

        // Parse expiration from JWT claims `exp` field (Unix timestamp)
        let expires_at = api_result
            .claims
            .exp
            .and_then(|exp| time::OffsetDateTime::from_unix_timestamp(exp).ok());

        // Build result
        let license_id_str = api_result.claims.license_id.clone();
        let state = if api_result.is_valid {
            LicenseCheckState::Valid
        } else {
            LicenseCheckState::Invalid
        };

        let verified_at = time::OffsetDateTime::now_utc();
        let error_message = api_result.error_message.clone();

        // Build result
        let result = LicenseCheckResult {
            state,
            company: Some(api_result.claims.company),
            license_type: Some(api_result.claims.license_type.to_string()),
            license_id: Some(license_id_str),
            issued_at: Some(issued_at),
            expires_at,
            version: Some(api_result.claims.version),
            verified_at,
            error_message,
        };

        // Cache the result
        Self::cache_verification_result(&result, &cache_manager).await;

        Ok(result)
    }

    /// Cache a verification result (shared logic for both success and error)
    async fn cache_verification_result(
        result: &LicenseCheckResult,
        cache_manager: &Arc<CacheManager>,
    ) {
        // Create result matching LicenseVerificationResult format for cache
        // This must match exactly what LicenseService stores
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum CachedLicenseState {
            None,
            Invalid,
            Error,
            Valid,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        struct CachedLicenseResult {
            state: CachedLicenseState,
            #[serde(skip_serializing_if = "Option::is_none")]
            company: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            license_type: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            license_id: Option<String>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                with = "time::serde::rfc3339::option"
            )]
            issued_at: Option<time::OffsetDateTime>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                with = "time::serde::rfc3339::option"
            )]
            expires_at: Option<time::OffsetDateTime>,
            #[serde(skip_serializing_if = "Option::is_none")]
            version: Option<String>,
            #[serde(with = "time::serde::rfc3339")]
            verified_at: time::OffsetDateTime,
            #[serde(skip_serializing_if = "Option::is_none")]
            error_message: Option<String>,
        }

        let license_id = result.license_id.as_deref().unwrap_or("");

        let cached_state = match result.state {
            LicenseCheckState::None => CachedLicenseState::None,
            LicenseCheckState::Invalid => CachedLicenseState::Invalid,
            LicenseCheckState::Error => CachedLicenseState::Error,
            LicenseCheckState::Valid => CachedLicenseState::Valid,
        };

        let cached_result = CachedLicenseResult {
            state: cached_state,
            company: result.company.clone(),
            license_type: result.license_type.clone(),
            license_id: result.license_id.clone(),
            issued_at: result.issued_at,
            expires_at: result.expires_at,
            version: result.version.clone(),
            verified_at: result.verified_at,
            error_message: result.error_message.clone(),
        };

        let cache_key = format!("license:verification:{license_id}");
        let ttl = Some(86400_u64); // 24 hours (same as service)
        let _ = cache_manager
            .set(&cache_key, &cached_result, ttl)
            .await
            .map_err(|e| {
                eprintln!("Warning: Failed to cache license verification result: {e}");
            });
    }

    /// Extract `license_id` from JWT (same logic as `LicenseService::decode_license_key`)
    fn extract_license_id(
        license_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let parts: Vec<&str> = license_key.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }

        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload)?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded)?;

        claims
            .get("license_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| "Missing license_id in JWT claims".into())
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
