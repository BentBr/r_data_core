#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::models::LicenseClaims;

/// License verification API response
#[derive(Debug, Deserialize)]
struct LicenseVerificationResponse {
    /// Whether the license is valid
    valid: bool,
    /// Optional message
    #[serde(default)]
    message: Option<String>,
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

    // Parse response
    let api_result: LicenseVerificationResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {e}"))?;

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

/// Decode license claims from JWT without verification (for display purposes)
pub(crate) fn decode_license_claims(
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
