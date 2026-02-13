use std::collections::HashMap;

use actix_web::HttpRequest;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Trait for authentication providers
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Apply authentication to HTTP request builder
    ///
    /// # Errors
    /// Returns an error if authentication cannot be applied to the request.
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder>;

    /// Extract auth from incoming request (for pre-shared keys, etc.)
    ///
    /// # Errors
    /// Returns an error if extraction fails.
    fn extract_from_request(
        &self,
        req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>>;

    /// Auth type identifier
    fn auth_type(&self) -> &'static str;
}

/// Factory for creating auth providers
pub trait AuthFactory: Send + Sync {
    fn auth_type(&self) -> &'static str;
    /// # Errors
    /// Returns an error if the auth provider cannot be created from the config.
    fn create(
        &self,
        config: &serde_json::Value,
    ) -> r_data_core_core::error::Result<Box<dyn AuthProvider>>;
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
    BasicAuth { username: String, password: String },
    /// Pre-shared key (for provider workflow)
    PreSharedKey {
        key: String,
        location: KeyLocation,
        field_name: String,
    },
    /// Entity JWT authentication (for headless CMS endpoints)
    EntityJwt {
        /// Optional required claims: claim path → expected value
        #[serde(default, skip_serializing_if = "Option::is_none")]
        required_claims: Option<HashMap<String, serde_json::Value>>,
    },
}

fn default_api_key_header() -> String {
    "X-API-Key".to_string()
}

/// No-op auth provider
pub struct NoAuthProvider;

#[async_trait]
impl AuthProvider for NoAuthProvider {
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder> {
        Ok(builder)
    }

    fn extract_from_request(
        &self,
        _req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>> {
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
    #[must_use]
    pub fn new(key: String, header_name: Option<String>) -> Self {
        Self {
            key,
            header_name: header_name.unwrap_or_else(|| "X-API-Key".to_string()),
        }
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuthProvider {
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder> {
        Ok(builder.header(&self.header_name, &self.key))
    }

    fn extract_from_request(
        &self,
        _req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>> {
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
    #[must_use]
    pub const fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

#[async_trait]
impl AuthProvider for BasicAuthProvider {
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder> {
        Ok(builder.basic_auth(&self.username, Some(&self.password)))
    }

    fn extract_from_request(
        &self,
        _req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>> {
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
    #[must_use]
    pub const fn new(key: String, location: KeyLocation, field_name: String) -> Self {
        Self {
            key,
            location,
            field_name,
        }
    }
}

#[async_trait]
impl AuthProvider for PreSharedKeyAuthProvider {
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder> {
        match self.location {
            KeyLocation::Header => Ok(builder.header(&self.field_name, &self.key)),
            KeyLocation::Body => {
                // Body auth is handled separately during request building
                Ok(builder)
            }
        }
    }

    fn extract_from_request(
        &self,
        req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>> {
        match self.location {
            KeyLocation::Header => req.headers().get(&self.field_name).map_or_else(
                || Ok(None),
                |header_value| {
                    header_value
                        .to_str()
                        .map(|s| Some(s.to_string()))
                        .map_err(|e| {
                            r_data_core_core::error::Error::Validation(format!(
                                "Invalid header value: {e}"
                            ))
                        })
                },
            ),
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

/// Entity JWT auth provider (verification is done in the API layer, not here)
pub struct EntityJwtAuthProvider {
    required_claims: Option<HashMap<String, serde_json::Value>>,
}

impl EntityJwtAuthProvider {
    #[must_use]
    pub const fn new(required_claims: Option<HashMap<String, serde_json::Value>>) -> Self {
        Self { required_claims }
    }

    /// Get the required claims configuration
    #[must_use]
    pub const fn required_claims(&self) -> &Option<HashMap<String, serde_json::Value>> {
        &self.required_claims
    }
}

#[async_trait]
impl AuthProvider for EntityJwtAuthProvider {
    fn apply_to_request(
        &self,
        builder: RequestBuilder,
    ) -> r_data_core_core::error::Result<RequestBuilder> {
        // Entity JWT is validated on incoming requests, not applied to outgoing ones
        Ok(builder)
    }

    fn extract_from_request(
        &self,
        req: &HttpRequest,
    ) -> r_data_core_core::error::Result<Option<String>> {
        // Extract Bearer token from Authorization header
        req.headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map_or(Ok(None), |token| Ok(Some(token.to_string())))
    }

    fn auth_type(&self) -> &'static str {
        "entity_jwt"
    }
}

/// Create auth provider from config
///
/// # Errors
/// Returns an error if the auth provider cannot be created from the config.
pub fn create_auth_provider(
    config: &AuthConfig,
) -> r_data_core_core::error::Result<Box<dyn AuthProvider>> {
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
        AuthConfig::EntityJwt { required_claims } => Ok(Box::new(EntityJwtAuthProvider::new(
            required_claims.clone(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_jwt_config_serialization() {
        let config = AuthConfig::EntityJwt {
            required_claims: None,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["type"], "entity_jwt");

        let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
        match deserialized {
            AuthConfig::EntityJwt { required_claims } => {
                assert!(required_claims.is_none());
            }
            _ => panic!("Expected EntityJwt variant"),
        }
    }

    #[test]
    fn test_entity_jwt_config_with_required_claims() {
        let mut claims = HashMap::new();
        claims.insert(
            "extra.role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );
        let config = AuthConfig::EntityJwt {
            required_claims: Some(claims),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["type"], "entity_jwt");
        assert_eq!(json["required_claims"]["extra.role"], "admin");

        let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
        match deserialized {
            AuthConfig::EntityJwt { required_claims } => {
                let claims = required_claims.unwrap();
                assert_eq!(
                    claims.get("extra.role"),
                    Some(&serde_json::Value::String("admin".to_string()))
                );
            }
            _ => panic!("Expected EntityJwt variant"),
        }
    }

    #[test]
    fn test_entity_jwt_config_without_required_claims_omits_field() {
        let config = AuthConfig::EntityJwt {
            required_claims: None,
        };
        let json = serde_json::to_value(&config).unwrap();
        // required_claims should be omitted when None (skip_serializing_if)
        assert!(!json.as_object().unwrap().contains_key("required_claims"));
    }
}
