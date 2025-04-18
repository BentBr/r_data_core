use crate::api::jwt::AuthUserClaims;
use crate::api::ApiState;
use actix_web::{
    dev::Payload, error::ErrorUnauthorized, web, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{ready, Ready};
use log::debug;

/// Extractor for required authentication
pub struct RequiredAuth(pub AuthUserClaims);

/// Extractor for optional authentication
pub struct OptionalAuth(pub Option<AuthUserClaims>);

impl FromRequest for RequiredAuth {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!("Handling required authentication FromRequest");

        // Check for claims in request extensions
        if let Some(claims) = req.extensions().get::<AuthUserClaims>().cloned() {
            ready(Ok(RequiredAuth(claims)))
        } else {
            // Try to extract from authorization header
            if let Some(state) = req.app_data::<web::Data<ApiState>>() {
                if let Some(auth_header) = req.headers().get("Authorization") {
                    if let Ok(auth_str) = auth_header.to_str() {
                        if auth_str.starts_with("Bearer ") {
                            let token = &auth_str[7..]; // Remove "Bearer " prefix
                            match crate::api::jwt::verify_jwt(token, &state.jwt_secret) {
                                Ok(claims) => {
                                    return ready(Ok(RequiredAuth(claims)));
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            ready(Err(ErrorUnauthorized("Authentication required")))
        }
    }
}

impl FromRequest for OptionalAuth {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        debug!("Handling optional authentication FromRequest");

        // Check for claims in request extensions
        if let Some(claims) = req.extensions_mut().get::<AuthUserClaims>().cloned() {
            return ready(Ok(OptionalAuth(Some(claims))));
        }

        // Try to extract from the authorization header
        if let Some(state) = req.app_data::<web::Data<ApiState>>() {
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..]; // Remove "Bearer " prefix
                        if let Ok(claims) = crate::api::jwt::verify_jwt(token, &state.jwt_secret) {
                            return ready(Ok(OptionalAuth(Some(claims))));
                        }
                    }
                }
            }
        }

        // Return None if no auth found, but don't fail
        ready(Ok(OptionalAuth(None)))
    }
}
