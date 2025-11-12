use actix_web::HttpRequest;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Trait for authentication providers
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Apply authentication to HTTP request builder
    fn apply_to_request(&self, builder: RequestBuilder) -> Result<RequestBuilder>;

    /// Extract auth from incoming request (for pre-shared keys, etc.)
    fn extract_from_request(&self, req: &HttpRequest) -> Result<Option<String>>;

    /// Auth type identifier
    fn auth_type(&self) -> &'static str;
}

/// Factory for creating auth providers
pub trait AuthFactory: Send + Sync {
    fn auth_type(&self) -> &'static str;
    fn create(&self, config: &serde_json::Value) -> Result<Box<dyn AuthProvider>>;
}

/// Key location for pre-shared keys
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KeyLocation {
    Header,
    Body,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// No authentication
    None,
    /// External API key
    ApiKey {
        /// API key value (for external) or UUID (for internal)
        key: String,
        /// Header name (default: "X-API-Key")
        #[serde(default = "default_api_key_header")]
        header_name: String,
    },
    /// Basic authentication
    BasicAuth {
        username: String,
        password: String,
    },
    /// Pre-shared key (for provider workflow)
    PreSharedKey {
        key: String,
        location: KeyLocation,
        field_name: String,
    },
}

fn default_api_key_header() -> String {
    "X-API-Key".to_string()
}

/// No-op auth provider
pub struct NoAuthProvider;

#[async_trait]
impl AuthProvider for NoAuthProvider {
    fn apply_to_request(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        Ok(builder)
    }

    fn extract_from_request(&self, _req: &HttpRequest) -> Result<Option<String>> {
        Ok(None)
    }

    fn auth_type(&self) -> &'static str {
        "none"
    }
}

/// API key auth provider (external)
pub struct ApiKeyAuthProvider {
    key: String,
    header_name: String,
}

impl ApiKeyAuthProvider {
    pub fn new(key: String, header_name: Option<String>) -> Self {
        Self {
            key,
            header_name: header_name.unwrap_or_else(|| "X-API-Key".to_string()),
        }
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuthProvider {
    fn apply_to_request(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        Ok(builder.header(&self.header_name, &self.key))
    }

    fn extract_from_request(&self, _req: &HttpRequest) -> Result<Option<String>> {
        Ok(None)
    }

    fn auth_type(&self) -> &'static str {
        "api_key"
    }
}

/// Basic auth provider
pub struct BasicAuthProvider {
    username: String,
    password: String,
}

impl BasicAuthProvider {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

#[async_trait]
impl AuthProvider for BasicAuthProvider {
    fn apply_to_request(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        Ok(builder.basic_auth(&self.username, Some(&self.password)))
    }

    fn extract_from_request(&self, _req: &HttpRequest) -> Result<Option<String>> {
        Ok(None)
    }

    fn auth_type(&self) -> &'static str {
        "basic_auth"
    }
}

/// Pre-shared key auth provider (for provider workflow)
pub struct PreSharedKeyAuthProvider {
    key: String,
    location: KeyLocation,
    field_name: String,
}

impl PreSharedKeyAuthProvider {
    pub fn new(key: String, location: KeyLocation, field_name: String) -> Self {
        Self {
            key,
            location,
            field_name,
        }
    }
}

#[async_trait]
impl AuthProvider for PreSharedKeyAuthProvider {
    fn apply_to_request(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        match self.location {
            KeyLocation::Header => Ok(builder.header(&self.field_name, &self.key)),
            KeyLocation::Body => {
                // Body auth is handled separately during request building
                Ok(builder)
            }
        }
    }

    fn extract_from_request(&self, req: &HttpRequest) -> Result<Option<String>> {
        match self.location {
            KeyLocation::Header => {
                if let Some(header_value) = req.headers().get(&self.field_name) {
                    Ok(Some(header_value.to_str()?.to_string()))
                } else {
                    Ok(None)
                }
            }
            KeyLocation::Body => {
                // Body extraction needs to be done in the route handler
                // This is a placeholder
                Ok(None)
            }
        }
    }

    fn auth_type(&self) -> &'static str {
        "pre_shared_key"
    }
}

/// Create auth provider from config
pub fn create_auth_provider(config: &AuthConfig) -> Result<Box<dyn AuthProvider>> {
    match config {
        AuthConfig::None => Ok(Box::new(NoAuthProvider)),
        AuthConfig::ApiKey { key, header_name } => Ok(Box::new(ApiKeyAuthProvider::new(
            key.clone(),
            Some(header_name.clone()),
        ))),
        AuthConfig::BasicAuth { username, password } => Ok(Box::new(BasicAuthProvider::new(
            username.clone(),
            password.clone(),
        ))),
        AuthConfig::PreSharedKey {
            key,
            location,
            field_name,
        } => Ok(Box::new(PreSharedKeyAuthProvider::new(
            key.clone(),
            location.clone(),
            field_name.clone(),
        ))),
    }
}

