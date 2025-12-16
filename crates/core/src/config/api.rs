#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API host
    pub host: String,

    /// API port
    pub port: u16,

    /// Enable SSL/TLS
    pub use_tls: bool,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// JWT token expiration in seconds
    pub jwt_expiration: u64,

    /// Enable documentation
    pub enable_docs: bool,

    /// CORS allowed origins
    pub cors_origins: Vec<String>,

    /// Check if default admin password is still in use
    pub check_default_admin_password: bool,
}
