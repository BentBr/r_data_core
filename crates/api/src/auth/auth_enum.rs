use actix_web::{
    dev::Payload, error::ErrorUnauthorized, web, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{ready, Ready};
use log::debug;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::jwt::AuthUserClaims;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::{ApiKeyInfo, extract_and_validate_api_key};

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

/// Extract and verify JWT from the Authorization header
fn extract_jwt_from_request(req: &HttpRequest) -> Option<AuthUserClaims> {
    if let Some(state) = req.app_data::<actix_web::web::Data<ApiStateWrapper>>() {
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..]; // Remove "Bearer " prefix
                    match crate::jwt::verify_jwt(token, state.jwt_secret()) {
                        Ok(claims) => {
                            debug!("JWT validation successful for user: {}", claims.name);
                            return Some(claims);
                        }
                        Err(e) => {
                            debug!("JWT validation failed: {:?}", e);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Safely get JWT claims from request by first checking extensions
fn get_or_validate_jwt(req: &HttpRequest) -> Option<AuthUserClaims> {
    // First check extensions without modifying them
    if let Some(claims) = req.extensions().get::<AuthUserClaims>() {
        return Some(claims.clone());
    }

    // If not found in extensions, try to extract from header
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

        match get_or_validate_jwt(req) {
            Some(claims) => ready(Ok(RequiredAuth(claims))),
            None => ready(Err(actix_web::error::ErrorUnauthorized("Authentication required"))),
        }
    }
}

impl RequiredAuth {
    /// Returns the authenticated user's UUID parsed from JWT claims subject.
    /// None if parsing fails.
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
        ready(Ok(OptionalAuth(claims)))
    }
}

/// Extractor for combined required authentication (JWT, API key, or pre-shared key)
pub struct CombinedRequiredAuth {
    pub jwt_claims: Option<AuthUserClaims>,
    pub api_key_info: Option<ApiKeyInfo>,
    pub pre_shared_key_valid: bool,
}

impl FromRequest for CombinedRequiredAuth {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!("Handling combined required authentication FromRequest");

        let req = req.clone();

        Box::pin(async move {
            // Check for JWT auth first
            if let Some(jwt_claims) = get_or_validate_jwt(&req) {
                return Ok(CombinedRequiredAuth {
                    jwt_claims: Some(jwt_claims),
                    api_key_info: None,
                    pre_shared_key_valid: false,
                });
            }

            // Check for API key in extensions
            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                return Ok(CombinedRequiredAuth {
                    jwt_claims: None,
                    api_key_info: Some(api_key_info.clone()),
                    pre_shared_key_valid: false,
                });
            }

            // Try API key authentication from headers
            if req.app_data::<web::Data<ApiStateWrapper>>().is_some() {
                // Check for an API key in the X-API-Key header
                if let Some(_) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
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

                            return Ok(CombinedRequiredAuth {
                                jwt_claims: None,
                                api_key_info: Some(ApiKeyInfo {
                                    uuid: key_uuid,
                                    user_uuid,
                                    name: key.name.clone(),
                                    created_at: key.created_at,
                                    expires_at: key.expires_at,
                                }),
                                pre_shared_key_valid: false,
                            });
                        }
                        Ok(None) => {
                            debug!("API key not found or invalid");
                        }
                        Err(e) => {
                            debug!("API key validation error: {:?}", e);
                        }
                    }
                }
            }

            // Check for pre-shared key in extensions (set by middleware or route handler)
            if let Some(valid) = req.extensions().get::<bool>() {
                if *valid {
                    return Ok(CombinedRequiredAuth {
                        jwt_claims: None,
                        api_key_info: None,
                        pre_shared_key_valid: true,
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
