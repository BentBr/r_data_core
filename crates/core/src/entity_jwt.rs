use std::collections::HashMap;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::error::{Error, Result};

/// Issuer claim for entity JWTs — distinct from admin JWTs
pub const ENTITY_JWT_ISSUER: &str = "r_data_core_entity";

/// Suffix appended to the base JWT secret to derive the entity-specific signing key
const ENTITY_JWT_SECRET_SUFFIX: &str = "_entity";

/// Claims encoded in entity-issued JWTs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAuthClaims {
    /// Entity UUID (subject)
    pub sub: String,
    /// Issuer — always `ENTITY_JWT_ISSUER`
    pub iss: String,
    /// Entity type (e.g. "user")
    pub entity_type: String,
    /// Additional claims copied from entity fields (e.g. role, tier)
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
    /// Expiration (seconds since epoch)
    pub exp: usize,
    /// Issued-at (seconds since epoch)
    pub iat: usize,
}

/// Derive the entity JWT signing secret from the base JWT secret.
#[must_use]
pub fn entity_jwt_secret(base_secret: &str) -> String {
    format!("{base_secret}{ENTITY_JWT_SECRET_SUFFIX}")
}

/// Generate a signed entity JWT.
///
/// # Errors
/// Returns `Error::Auth` if token encoding fails.
#[allow(clippy::implicit_hasher)] // Internal function, always called with default HashMap
pub fn generate_entity_jwt(
    entity_uuid: &str,
    entity_type: &str,
    extra: HashMap<String, serde_json::Value>,
    base_jwt_secret: &str,
    expiry_secs: u64,
) -> Result<String> {
    let now = OffsetDateTime::now_utc();
    let expiration = now
        .checked_add(Duration::seconds(
            i64::try_from(expiry_secs).unwrap_or(i64::MAX),
        ))
        .ok_or_else(|| Error::Auth("Could not create token expiration".to_string()))?;

    let claims = EntityAuthClaims {
        sub: entity_uuid.to_string(),
        iss: ENTITY_JWT_ISSUER.to_string(),
        entity_type: entity_type.to_string(),
        extra,
        exp: usize::try_from(expiration.unix_timestamp()).unwrap_or(0),
        iat: usize::try_from(now.unix_timestamp()).unwrap_or(0),
    };

    let secret = entity_jwt_secret(base_jwt_secret);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| Error::Auth(format!("Entity token generation error: {e}")))
}

/// Verify and decode an entity JWT.
///
/// Checks that the signing key matches and the issuer is `ENTITY_JWT_ISSUER`.
///
/// # Errors
/// Returns `Error::Auth` if verification fails or the issuer is unexpected.
pub fn verify_entity_jwt(token: &str, base_jwt_secret: &str) -> Result<EntityAuthClaims> {
    let secret = entity_jwt_secret(base_jwt_secret);
    let validation = Validation::default();

    let token_data = decode::<EntityAuthClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| Error::Auth(format!("Entity token validation error: {e}")))?;

    // Verify issuer
    if token_data.claims.iss != ENTITY_JWT_ISSUER {
        return Err(Error::Auth(format!(
            "Unexpected issuer: {}",
            token_data.claims.iss
        )));
    }

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test_jwt_secret";

    #[test]
    fn test_entity_jwt_secret_suffix() {
        assert_eq!(entity_jwt_secret("base"), "base_entity");
    }

    #[test]
    fn test_generate_and_verify_round_trip() {
        let extra = HashMap::from([(
            "role".to_string(),
            serde_json::Value::String("member".to_string()),
        )]);

        let token = generate_entity_jwt("uuid-123", "user", extra, TEST_SECRET, 3600).unwrap();

        let claims = verify_entity_jwt(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.sub, "uuid-123");
        assert_eq!(claims.entity_type, "user");
        assert_eq!(claims.iss, ENTITY_JWT_ISSUER);
        assert_eq!(
            claims.extra.get("role"),
            Some(&serde_json::Value::String("member".to_string()))
        );
    }

    #[test]
    fn test_wrong_base_secret_fails() {
        let token =
            generate_entity_jwt("uuid-123", "user", HashMap::new(), TEST_SECRET, 3600).unwrap();

        let result = verify_entity_jwt(&token, "wrong_secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_jwt_cannot_verify_as_entity_jwt() {
        // An admin JWT is signed with the base secret (no suffix), so entity
        // verification (which uses base + suffix) should reject it.
        let admin_token = jsonwebtoken::encode(
            &Header::default(),
            &EntityAuthClaims {
                sub: "admin-uuid".to_string(),
                iss: "r_data_core_admin".to_string(),
                entity_type: String::new(),
                extra: HashMap::new(),
                exp: usize::try_from(
                    (OffsetDateTime::now_utc() + Duration::hours(1)).unix_timestamp(),
                )
                .unwrap_or(0),
                iat: usize::try_from(OffsetDateTime::now_utc().unix_timestamp()).unwrap_or(0),
            },
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();

        let result = verify_entity_jwt(&admin_token, TEST_SECRET);
        assert!(result.is_err());
    }
}
