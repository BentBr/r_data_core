use actix_web::HttpRequest;

use crate::jwt::AuthUserClaims;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

#[allow(dead_code)]
fn extract_jwt_from_request(_req: &HttpRequest) -> Option<AuthUserClaims> {
    None
}

#[allow(dead_code)]
fn get_or_validate_jwt(_req: &HttpRequest) -> Option<AuthUserClaims> {
    None
}
