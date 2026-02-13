use actix_web::{
    dev::Payload, error::ErrorUnauthorized, web, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{ready, Ready};
use log::debug;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::{extract_and_validate_api_key, extract_jwt_token_string, ApiKeyInfo};
use r_data_core_core::admin_jwt::AuthUserClaims;
use r_data_core_core::entity_jwt::EntityAuthClaims;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

/// Extract and verify JWT from the Authorization header
fn extract_jwt_from_request(req: &HttpRequest) -> Option<AuthUserClaims> {
    if let Some(state) = req.app_data::<web::Data<ApiStateWrapper>>() {
        if let Some(token) = extract_jwt_token_string(req) {
            match r_data_core_core::admin_jwt::verify_jwt(token, state.jwt_secret()) {
                Ok(claims) => {
                    let name = &claims.name;
                    debug!("JWT validation successful for user: {name}");
                    return Some(claims);
                }
                Err(e) => {
                    debug!("JWT validation failed: {e:?}");
                }
            }
        }
    }
    None
}

/// Safely get JWT claims from request by first checking extensions
fn get_or_validate_jwt(req: &HttpRequest) -> Option<AuthUserClaims> {
    // First, check extensions without modifying them
    if let Some(claims) = req.extensions().get::<AuthUserClaims>() {
        return Some(claims.clone());
    }

    // If not found in extensions, try to extract from the header
    extract_jwt_from_request(req)
}

/// Extractor for required authentication
#[derive(Debug)]
pub struct RequiredAuth(pub AuthUserClaims);

/// Extractor for optional authentication
pub struct OptionalAuth(pub Option<AuthUserClaims>);

impl FromRequest for RequiredAuth {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!("Handling required authentication FromRequest");

        get_or_validate_jwt(req).map_or_else(
            || ready(Err(ErrorUnauthorized("Authentication required"))),
            |claims| ready(Ok(Self(claims))),
        )
    }
}

impl RequiredAuth {
    /// Returns the authenticated user's UUID parsed from JWT claims subject.
    /// None if parsing fails.
    #[must_use]
    pub fn user_uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.0.sub).ok()
    }
}

impl FromRequest for OptionalAuth {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!("Handling optional authentication FromRequest");

        // Return option based on whether claims were found
        let claims = get_or_validate_jwt(req);
        ready(Ok(Self(claims)))
    }
}

/// Extractor for combined required authentication (JWT, API key, pre-shared key, or entity JWT)
pub struct CombinedRequiredAuth {
    pub jwt_claims: Option<AuthUserClaims>,
    pub api_key_info: Option<ApiKeyInfo>,
    pub pre_shared_key_valid: bool,
    pub entity_jwt_claims: Option<EntityAuthClaims>,
}

impl FromRequest for CombinedRequiredAuth {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!(
            "Handling combined required authentication FromRequest for path: {}",
            req.path()
        );

        let req = req.clone();

        Box::pin(async move {
            // Check for JWT auth first
            if let Some(jwt_claims) = get_or_validate_jwt(&req) {
                return Ok(Self {
                    jwt_claims: Some(jwt_claims),
                    api_key_info: None,
                    pre_shared_key_valid: false,
                    entity_jwt_claims: None,
                });
            }

            // Check for the API key in extensions
            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                return Ok(Self {
                    jwt_claims: None,
                    api_key_info: Some(api_key_info.clone()),
                    pre_shared_key_valid: false,
                    entity_jwt_claims: None,
                });
            }

            // Try API key authentication from headers
            if req.app_data::<web::Data<ApiStateWrapper>>().is_some() {
                debug!("Found ApiStateWrapper in app_data");
                // Check for an API key in the X-API-Key header
                if let Some(api_key_header) =
                    req.headers().get("X-API-Key").and_then(|h| h.to_str().ok())
                {
                    debug!("Found X-API-Key header, length: {}", api_key_header.len());
                    // Try to validate an API key
                    match extract_and_validate_api_key(&req).await {
                        Ok(Some((key, user_uuid))) => {
                            let key_uuid = key.uuid;
                            req.extensions_mut().insert(ApiKeyInfo {
                                uuid: key_uuid,
                                user_uuid,
                                name: key.name.clone(),
                                created_at: key.created_at,
                                expires_at: key.expires_at,
                            });

                            return Ok(Self {
                                jwt_claims: None,
                                api_key_info: Some(ApiKeyInfo {
                                    uuid: key_uuid,
                                    user_uuid,
                                    name: key.name.clone(),
                                    created_at: key.created_at,
                                    expires_at: key.expires_at,
                                }),
                                pre_shared_key_valid: false,
                                entity_jwt_claims: None,
                            });
                        }
                        Ok(None) => {
                            debug!("API key not found or invalid");
                        }
                        Err(e) => {
                            debug!("API key validation error: {e:?}");
                        }
                    }
                }
            }

            // Check for pre-shared key or entity JWT in extensions (set by middleware or route handler)
            if let Some(valid) = req.extensions().get::<bool>() {
                if *valid {
                    // Check if entity JWT claims were stored by validate_provider_auth
                    let entity_claims = req.extensions().get::<EntityAuthClaims>().cloned();
                    return Ok(Self {
                        jwt_claims: None,
                        api_key_info: None,
                        pre_shared_key_valid: true,
                        entity_jwt_claims: entity_claims,
                    });
                }
            }

            // All authentication methods failed
            Err(ErrorUnauthorized(
                "Authentication required. Please provide a valid JWT token, API key, or pre-shared key.",
            ))
        })
    }
}

impl CombinedRequiredAuth {
    /// Get user UUID from either JWT claims or API key info
    #[must_use]
    pub fn get_user_uuid(&self) -> Option<Uuid> {
        // Extract from API key information
        if let Some(api_key_info) = &self.api_key_info {
            return Some(api_key_info.user_uuid);
        }

        // Or extract from JWT claims
        if let Some(claims) = &self.jwt_claims {
            if let Ok(uuid) = Uuid::parse_str(&claims.sub) {
                return Some(uuid);
            }
        }

        None
    }
}
