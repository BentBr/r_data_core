#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::LicenseConfig;
use r_data_core_core::error::Result;
use r_data_core_license::{
    call_verification_api, decode_license_claims, LICENSE_CACHE_KEY_PREFIX, LICENSE_CACHE_TTL_SECS,
};

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

impl LicenseVerificationResult {
    /// Create a "no license" result
    #[must_use]
    pub fn none() -> Self {
        Self {
            state: LicenseState::None,
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
}

/// Service for license verification with caching
pub struct LicenseService {
    /// License configuration
    pub config: LicenseConfig,
    /// HTTP client for API calls (kept for potential future use)
    #[allow(dead_code)]
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

    /// Build cache key for a license ID
    fn cache_key(license_id: &str) -> String {
        format!("{LICENSE_CACHE_KEY_PREFIX}{license_id}")
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
            _ => return Ok(LicenseVerificationResult::none()),
        };

        // Try to decode license key locally first to get license_id
        let license_id = decode_license_claims(license_key)
            .ok()
            .map(|claims| claims.license_id);

        // Check cache first
        if let Some(ref license_id) = license_id {
            let cache_key = Self::cache_key(license_id);
            if let Ok(Some(cached)) = self
                .cache
                .get::<LicenseVerificationResult>(&cache_key)
                .await
            {
                return Ok(cached);
            }
        }

        // Call verification API using the shared implementation
        let verification_result = self
            .call_verification_api_internal(license_key, license_id.as_deref())
            .await;

        // Cache the result if we have a license_id
        if let Some(ref license_id) = license_id {
            let cache_key = Self::cache_key(license_id);
            let ttl = Some(LICENSE_CACHE_TTL_SECS);
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
            _ => return Ok(Some(LicenseVerificationResult::none())),
        };

        // Try to decode license key to get license_id
        let license_id = match decode_license_claims(license_key) {
            Ok(claims) => claims.license_id,
            Err(_) => return Ok(Some(LicenseVerificationResult::none())),
        };

        // Get from cache
        let cache_key = Self::cache_key(&license_id);
        self.cache
            .get::<LicenseVerificationResult>(&cache_key)
            .await
    }

    /// Call the license verification API using the shared implementation
    async fn call_verification_api_internal(
        &self,
        license_key: &str,
        license_id: Option<&str>,
    ) -> LicenseVerificationResult {
        // Use the shared API call implementation from r_data_core_license
        match call_verification_api(license_key, &self.config.verification_url).await {
            Ok(api_result) => {
                // Convert API result to service result
                let state = if api_result.is_valid {
                    LicenseState::Valid
                } else {
                    LicenseState::Invalid
                };

                LicenseVerificationResult {
                    state,
                    company: Some(api_result.claims.company),
                    license_type: Some(api_result.claims.license_type.to_string()),
                    license_id: Some(api_result.claims.license_id),
                    issued_at: Self::parse_issued_at(&api_result.claims.issued_at),
                    expires_at: Self::parse_expiration(api_result.claims.exp),
                    version: Some(api_result.claims.version),
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: api_result.error_message,
                }
            }
            Err(e) => {
                log::error!("License verification API call failed: {e}");

                // Try to get claims for partial info in error result
                let claims = decode_license_claims(license_key).ok();

                LicenseVerificationResult {
                    state: LicenseState::Error,
                    company: claims.as_ref().map(|c| c.company.clone()),
                    license_type: claims.as_ref().map(|c| c.license_type.to_string()),
                    license_id: license_id.map(str::to_string),
                    issued_at: claims
                        .as_ref()
                        .and_then(|c| Self::parse_issued_at(&c.issued_at)),
                    expires_at: claims.as_ref().and_then(|c| Self::parse_expiration(c.exp)),
                    version: claims.as_ref().map(|c| c.version.clone()),
                    verified_at: time::OffsetDateTime::now_utc(),
                    error_message: Some(e.to_string()),
                }
            }
        }
    }
}
