#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::models::LicenseClaims;

/// License verification API response (unwrapped format from external API)
#[derive(Debug, Deserialize)]
struct LicenseVerificationResponse {
    /// Whether the license is valid
    valid: bool,
    /// Optional message
    #[serde(default)]
    message: Option<String>,
}

/// `ApiResponse` wrapper (used by internal verification endpoint)
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Struct is used in generic type parameter for parsing
struct ApiResponseWrapper<T> {
    /// Response status
    status: String,
    /// Response message
    message: String,
    /// Response data
    data: Option<T>,
}

/// Result of license verification API call
#[derive(Debug, Clone)]
pub struct LicenseVerificationApiResult {
    /// Whether the license is valid
    pub is_valid: bool,
    /// License claims from the JWT
    pub claims: LicenseClaims,
    /// Error message (if invalid or error occurred)
    pub error_message: Option<String>,
}

/// Call the license verification API
///
/// This function performs the same API call as the maintenance task.
///
/// # Arguments
/// * `license_key` - License key JWT token
/// * `verification_url` - URL of the license verification API
///
/// # Errors
/// Returns an error if the API call fails
pub async fn call_verification_api(
    license_key: &str,
    verification_url: &str,
) -> Result<LicenseVerificationApiResult, Box<dyn std::error::Error + Send + Sync>> {
    // First, decode the license key to get claims
    let claims = decode_license_claims(license_key)?;

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    // Call API
    let response = client
        .post(verification_url)
        .json(&json!({ "license_key": license_key }))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    // Parse response - handle both wrapped (ApiResponse) and unwrapped formats
    let verification_result = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse API response: {e}"))?;

    // Try to parse as ApiResponse wrapper first (internal endpoint format)
    let api_result = if let Ok(wrapped) = serde_json::from_value::<
        ApiResponseWrapper<LicenseVerificationResponse>,
    >(verification_result.clone())
    {
        // Extract data from wrapped response
        wrapped
            .data
            .ok_or_else(|| "ApiResponse data field is missing".to_string())?
    } else {
        // Fall back to unwrapped format (external API format)
        serde_json::from_value::<LicenseVerificationResponse>(verification_result)
            .map_err(|e| format!("Failed to parse verification response: {e}"))?
    };

    Ok(LicenseVerificationApiResult {
        is_valid: api_result.valid,
        claims,
        error_message: if api_result.valid {
            None
        } else {
            api_result.message
        },
    })
}

/// Cache key prefix for license verification results
pub const LICENSE_CACHE_KEY_PREFIX: &str = "license:verification:";

/// Cache TTL for license verification results (24 hours in seconds)
pub const LICENSE_CACHE_TTL_SECS: u64 = 86400;

/// Decode license claims from JWT without verification (for display purposes)
///
/// # Arguments
/// * `license_key` - License key JWT token
///
/// # Errors
/// Returns an error if the JWT format is invalid or cannot be decoded
pub fn decode_license_claims(
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
