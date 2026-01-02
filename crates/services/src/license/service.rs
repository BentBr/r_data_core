#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::LicenseConfig;
use r_data_core_core::error::Result;
use r_data_core_license::LicenseClaims;

/// License verification state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseState {
    /// No license key provided
    None,
    /// License key is invalid (API returned valid=false)
    Invalid,
    /// Network/technical error during verification
    Error,
    /// License key is valid
    Valid,
}

/// Cached license verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseVerificationResult {
    /// License state
    pub state: LicenseState,
    /// Company name (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    /// License type (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_type: Option<String>,
    /// License ID (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    /// Issue date (if license is present)
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub issued_at: Option<time::OffsetDateTime>,
    /// Expiration date (if license is present and has expiration)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub expires_at: Option<time::OffsetDateTime>,
    /// License version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Verification timestamp
    #[serde(with = "time::serde::rfc3339")]
    pub verified_at: time::OffsetDateTime,
    /// Error message (only present if state is "error" or "invalid")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// License verification API response
#[derive(Debug, Deserialize)]
struct LicenseVerificationResponse {
    /// Whether the license is valid
    valid: bool,
    /// Optional message
    #[serde(default)]
    message: Option<String>,
}

/// Service for license verification with caching
pub struct LicenseService {
    /// License configuration
    pub config: LicenseConfig,
    /// HTTP client for API calls
    client: Client,
    /// Cache manager
    cache: Arc<CacheManager>,
}

impl LicenseService {
    /// Parse `issued_at` string to `OffsetDateTime`
    fn parse_issued_at(issued_at: &str) -> Option<time::OffsetDateTime> {
        time::OffsetDateTime::parse(issued_at, &time::format_description::well_known::Rfc3339).ok()
    }

    /// Parse expiration from JWT claims `exp` field (Unix timestamp)
    fn parse_expiration(exp: Option<i64>) -> Option<time::OffsetDateTime> {
        exp.and_then(|timestamp| time::OffsetDateTime::from_unix_timestamp(timestamp).ok())
    }

    /// Create a new license service
    ///
    /// # Arguments
    /// * `config` - License configuration
    /// * `cache` - Cache manager
    #[must_use]
    pub fn new(config: LicenseConfig, cache: Arc<CacheManager>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            config,
            client,
            cache,
        }
    }

    /// Verify license on startup and log the result
    ///
    /// This is a convenience method that verifies the license and logs the result
    /// without returning an error (startup should continue regardless of license status).
    ///
    /// # Arguments
    /// * `service_name` - Name of the service (e.g., "core", "worker", "maintenance") for logging
    pub async fn verify_license_on_startup(&self, service_name: &str) {
        match self.verify_license().await {
            Ok(result) => match result.state {
                LicenseState::Valid => {
                    log::info!("License verified successfully on {service_name} startup");
                }
                LicenseState::Invalid => {
                    log::warn!(
                        "License is invalid on {service_name} startup: {:?}",
                        result.error_message
                    );
                }
                LicenseState::Error => {
                    log::error!(
                        "License verification error on {service_name} startup: {:?}",
                        result.error_message
                    );
                }
                LicenseState::None => {
                    log::warn!("No license key configured on {service_name} startup");
                }
            },
            Err(e) => {
                log::error!("Failed to verify license on {service_name} startup: {e}");
            }
        }
    }

    /// Verify license key and cache the result
    ///
    /// # Errors
    /// Returns an error if cache operation fails (but not if API call fails - that's handled as Error state)
    pub async fn verify_license(&self) -> Result<LicenseVerificationResult> {
        // Check if license key is provided
        let license_key = match &self.config.license_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                return Ok(LicenseVerificationResult {
                    state: LicenseState::None,
                    company: None,
                    license_type: None,
                    license_id: None,
                    issued_at: None,
                    expires_at: None,
                    version: None,
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: None,
                });
            }
        };

        // Try to decode license key locally first to get license_id
        let license_id = match Self::decode_license_key(license_key) {
            Ok(claims) => Some(claims.license_id),
            Err(_) => None,
        };

        // Check cache first
        if let Some(license_id) = &license_id {
            let cache_key = format!("license:verification:{license_id}");
            if let Ok(Some(cached)) = self
                .cache
                .get::<LicenseVerificationResult>(&cache_key)
                .await
            {
                return Ok(cached);
            }
        }

        // Call verification API
        let verification_result = self.call_verification_api(license_key).await;

        // Cache the result if we have a license_id
        if let Some(license_id) = &license_id {
            let cache_key = format!("license:verification:{license_id}");
            let ttl = Some(86400_u64); // 24 hours
            let _ = self
                .cache
                .set(&cache_key, &verification_result, ttl)
                .await
                .map_err(|e| {
                    log::warn!("Failed to cache license verification result: {e}");
                });
        }

        Ok(verification_result)
    }

    /// Get cached license verification result (does not trigger new verification)
    ///
    /// # Errors
    /// Returns an error if cache operation fails
    pub async fn get_cached_license_status(&self) -> Result<Option<LicenseVerificationResult>> {
        let license_key = match &self.config.license_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                // No license key - return None state
                return Ok(Some(LicenseVerificationResult {
                    state: LicenseState::None,
                    company: None,
                    license_type: None,
                    license_id: None,
                    issued_at: None,
                    expires_at: None,
                    version: None,
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: None,
                }));
            }
        };

        // Try to decode license key to get license_id
        let license_id = match Self::decode_license_key(license_key) {
            Ok(claims) => claims.license_id,
            Err(_) => {
                // Can't decode - return None state
                return Ok(Some(LicenseVerificationResult {
                    state: LicenseState::None,
                    company: None,
                    license_type: None,
                    license_id: None,
                    issued_at: None,
                    expires_at: None,
                    version: None,
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: None,
                }));
            }
        };

        // Get from cache
        let cache_key = format!("license:verification:{license_id}");
        self.cache
            .get::<LicenseVerificationResult>(&cache_key)
            .await
    }

    /// Decode license key without verification (to extract `license_id`)
    fn decode_license_key(license_key: &str) -> Result<LicenseClaims> {
        // Try to decode JWT without verification to get claims
        // This is just to extract the license_id for caching
        use serde_json::Value;
        // Decode payload (base64url)
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let parts: Vec<&str> = license_key.split('.').collect();
        if parts.len() != 3 {
            return Err(r_data_core_core::error::Error::Validation(
                "Invalid JWT format".to_string(),
            ));
        }

        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to decode JWT: {e}"))
        })?;

        let claims: Value = serde_json::from_slice(&decoded).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Failed to parse JWT claims: {e}"))
        })?;

        // Extract license_id (we don't need it here, just validate the structure)
        let _license_id = claims
            .get("license_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                r_data_core_core::error::Error::Validation(
                    "Missing license_id in JWT claims".to_string(),
                )
            })?;

        // Try to parse as LicenseClaims
        serde_json::from_value(claims).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("Invalid license claims: {e}"))
        })
    }

    /// Call the license verification API
    async fn call_verification_api(&self, license_key: &str) -> LicenseVerificationResult {
        // First, try to decode the license key locally to get claims
        let claims = match Self::decode_license_key(license_key) {
            Ok(c) => c,
            Err(e) => {
                return LicenseVerificationResult {
                    state: LicenseState::Error,
                    company: None,
                    license_type: None,
                    license_id: None,
                    issued_at: None,
                    expires_at: None,
                    version: None,
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: Some(format!("Failed to decode license key: {e}")),
                };
            }
        };

        // Call API
        let response = match self
            .client
            .post(&self.config.verification_url)
            .json(&json!({ "license_key": license_key }))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("License verification API call failed: {e}");
                return LicenseVerificationResult {
                    state: LicenseState::Error,
                    company: Some(claims.company),
                    license_type: Some(claims.license_type.to_string()),
                    license_id: Some(claims.license_id),
                    issued_at: Self::parse_issued_at(&claims.issued_at),
                    expires_at: Self::parse_expiration(claims.exp),
                    version: Some(claims.version),
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: Some(format!("Network error: {e}")),
                };
            }
        };

        // Parse response
        let api_result = match response.json::<LicenseVerificationResponse>().await {
            Ok(result) => result,
            Err(e) => {
                log::error!("Failed to parse license verification response: {e}");
                return LicenseVerificationResult {
                    state: LicenseState::Error,
                    company: Some(claims.company),
                    license_type: Some(claims.license_type.to_string()),
                    license_id: Some(claims.license_id),
                    issued_at: Self::parse_issued_at(&claims.issued_at),
                    expires_at: Self::parse_expiration(claims.exp),
                    version: Some(claims.version),
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: Some(format!("Failed to parse API response: {e}")),
                };
            }
        };

        // Determine state based on API response
        let is_valid = api_result.valid;
        let state = if is_valid {
            LicenseState::Valid
        } else {
            LicenseState::Invalid
        };

        LicenseVerificationResult {
            state,
            company: Some(claims.company),
            license_type: Some(claims.license_type.to_string()),
            license_id: Some(claims.license_id),
            issued_at: Self::parse_issued_at(&claims.issued_at),
            expires_at: Self::parse_expiration(claims.exp),
            version: Some(claims.version),
            verified_at: time::OffsetDateTime::now_utc(),
            error_message: if is_valid { None } else { api_result.message },
        }
    }
}
