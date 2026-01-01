#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// License configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseConfig {
    /// License key JWT token (optional, but required for production)
    pub license_key: Option<String>,
    /// Private key for signing license keys (used by `license_tool` binary)
    pub private_key: Option<String>,
    /// License verification API URL
    #[serde(default = "default_license_verification_url")]
    pub verification_url: String,
    /// Statistics submission API URL
    #[serde(default = "default_statistics_url")]
    pub statistics_url: String,
}

fn default_license_verification_url() -> String {
    "https://license.rdatacore.eu/verify".to_string()
}

fn default_statistics_url() -> String {
    "https://statistics.rdatacore.eu/submit".to_string()
}

impl Default for LicenseConfig {
    fn default() -> Self {
        Self {
            license_key: None,
            private_key: None,
            verification_url: default_license_verification_url(),
            statistics_url: default_statistics_url(),
        }
    }
}
