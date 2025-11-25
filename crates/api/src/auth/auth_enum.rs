use actix_web::{
    dev::Payload, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{ready, Ready};
use log::debug;
use uuid::Uuid;

use crate::jwt::AuthUserClaims;
use crate::api_state::ApiStateTrait;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

/// Extract and verify JWT from the Authorization header
fn extract_jwt_from_request(req: &HttpRequest) -> Option<AuthUserClaims> {
    if let Some(state) = req.app_data::<actix_web::web::Data<dyn ApiStateTrait>>() {
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
