#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Deciding whether to believe an identity token.
//!
//! Everything here fails closed. The algorithm is chosen from an allowlist
//! rather than read from the token, the audience is checked rather than
//! assumed, and a key id that is not in the set is an error rather than a
//! reason to skip verification.
//!
//! `now` is a parameter rather than a call to the clock, so expiry can be
//! tested without sleeping.

use std::collections::HashMap;

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde_json::Value;
use thiserror::Error;

use super::config::OidcConfig;
use super::keys::{Jwk, JwkSet, OidcClaims};

/// Signature algorithms this system will verify.
///
/// Asymmetric only. A symmetric algorithm here would mean the verifier holds
/// the signing secret, and `none` would mean no verification at all — both are
/// classic JWT confusion attacks, so neither is reachable.
const ALLOWED_ALGORITHMS: &[Algorithm] = &[Algorithm::RS256, Algorithm::ES256];

/// Tolerance for clock drift between this host and the provider.
///
/// Sixty seconds is conventional. Much wider and expiry stops meaning much.
const CLOCK_SKEW_SECS: u64 = 60;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("the token is not a well-formed JWT: {0}")]
    Malformed(String),
    #[error("the token names no key id, so the signing key cannot be identified")]
    MissingKid,
    #[error("no signing key with id '{kid}'")]
    UnknownKid { kid: String },
    #[error(
        "signature algorithm '{alg}' is not accepted; this system verifies RS256 and ES256 only"
    )]
    UnsupportedAlgorithm { alg: String },
    #[error("the signing key with id '{kid}' is unusable: {reason}")]
    UnusableKey { kid: String, reason: String },
    #[error("the signature does not verify")]
    BadSignature,
    #[error("the token has expired")]
    Expired,
    #[error("the token is not valid yet")]
    NotYetValid,
    #[error("issued by '{found}', but this instance trusts '{expected}'")]
    IssuerMismatch { expected: String, found: String },
    #[error("audience '{expected}' is not among the token's, so it was minted for something else")]
    AudienceMismatch { expected: String },
    #[error("the token carries no subject, so there is no stable identity to key on")]
    MissingSubject,
    #[error("the token carries no expiry, which would make it a permanent credential")]
    MissingExpiry,
}

/// Verify a token and extract the claims this system reads.
///
/// # Errors
/// Returns `ValidationError` for any reason the token should not be trusted.
/// Callers should not distinguish between them when responding to a client —
/// the variants exist for logs and tests, not for telling an attacker which
/// part of their forgery was wrong.
pub fn validate_token(
    token: &str,
    keys: &JwkSet,
    config: &OidcConfig,
    now: i64,
) -> Result<OidcClaims, ValidationError> {
    let header = decode_header(token).map_err(|e| ValidationError::Malformed(e.to_string()))?;

    // The algorithm comes from the allowlist, not from the token. A token
    // asking to be verified with `none` gets rejected here rather than
    // accepted trivially.
    if !ALLOWED_ALGORITHMS.contains(&header.alg) {
        return Err(ValidationError::UnsupportedAlgorithm {
            alg: format!("{:?}", header.alg),
        });
    }

    let kid = header.kid.ok_or(ValidationError::MissingKid)?;
    let jwk = keys
        .find(&kid)
        .ok_or_else(|| ValidationError::UnknownKid { kid: kid.clone() })?;
    let key = decoding_key(jwk, &kid)?;

    // Narrowed to the header's algorithm, which the allowlist check above has
    // already approved. Passing the whole allowlist here fails: jsonwebtoken
    // will not verify against a list spanning RSA and EC families.
    let mut validation = Validation::new(header.alg);
    validation.algorithms = vec![header.alg];
    validation.set_issuer(&[config.issuer.as_str()]);
    validation.set_audience(&[config.audience.as_str()]);
    validation.leeway = CLOCK_SKEW_SECS;
    // Expiry is checked below against the caller's `now`, not against the
    // system clock. Leaving it to jsonwebtoken would make the `now` parameter
    // decorative — the signature would promise time injection while the real
    // clock quietly decided, which is a confusing kind of lie in security code.
    validation.validate_exp = false;
    validation.validate_nbf = false;

    let data = decode::<HashMap<String, Value>>(token, &key, &validation)
        .map_err(|e| classify(&e, config))?;

    claims_from(data.claims, config, now)
}

/// Turn a `jsonwebtoken` error into something a log can act on.
fn classify(error: &jsonwebtoken::errors::Error, config: &OidcConfig) -> ValidationError {
    use jsonwebtoken::errors::ErrorKind;
    match error.kind() {
        ErrorKind::ExpiredSignature => ValidationError::Expired,
        ErrorKind::ImmatureSignature => ValidationError::NotYetValid,
        ErrorKind::InvalidSignature => ValidationError::BadSignature,
        ErrorKind::InvalidIssuer => ValidationError::IssuerMismatch {
            expected: config.issuer.clone(),
            // The claimed issuer is not surfaced: it is attacker-controlled
            // text, and echoing it into a log invites injection.
            found: "a different issuer".to_string(),
        },
        ErrorKind::InvalidAudience => ValidationError::AudienceMismatch {
            expected: config.audience.clone(),
        },
        ErrorKind::InvalidAlgorithm => ValidationError::UnsupportedAlgorithm {
            alg: "rejected by the allowlist".to_string(),
        },
        // jsonwebtoken requires `exp` by default and reports it here, before
        // the check below ever runs. Map it to the specific error rather than
        // letting a missing expiry surface as generic malformedness.
        ErrorKind::MissingRequiredClaim(claim) if claim == "exp" => ValidationError::MissingExpiry,
        other => ValidationError::Malformed(format!("{other:?}")),
    }
}

/// Build a verification key from a JWK.
fn decoding_key(jwk: &Jwk, kid: &str) -> Result<DecodingKey, ValidationError> {
    let unusable = |reason: &str| ValidationError::UnusableKey {
        kid: kid.to_string(),
        reason: reason.to_string(),
    };

    match jwk.kty.as_str() {
        "RSA" => {
            let (Some(n), Some(e)) = (jwk.n.as_deref(), jwk.e.as_deref()) else {
                return Err(unusable("an RSA key needs both n and e"));
            };
            DecodingKey::from_rsa_components(n, e).map_err(|e| unusable(&e.to_string()))
        }
        "EC" => {
            let (Some(x), Some(y)) = (jwk.x.as_deref(), jwk.y.as_deref()) else {
                return Err(unusable("an EC key needs both x and y"));
            };
            DecodingKey::from_ec_components(x, y).map_err(|e| unusable(&e.to_string()))
        }
        other => Err(unusable(&format!(
            "key type '{other}' is not supported; RSA and EC only"
        ))),
    }
}

/// Extract the claims this system reads, ignoring everything else.
fn claims_from(
    raw: HashMap<String, Value>,
    config: &OidcConfig,
    now: i64,
) -> Result<OidcClaims, ValidationError> {
    // Time is checked here rather than in jsonwebtoken, so `now` is
    // authoritative. See the comment where validate_exp is disabled.
    let skew = i64::try_from(CLOCK_SKEW_SECS).unwrap_or(0);

    // A token with no expiry never stops being valid. Refuse it rather than
    // treat the absence as permission.
    let exp = raw
        .get("exp")
        .and_then(Value::as_i64)
        .ok_or(ValidationError::MissingExpiry)?;
    if now > exp + skew {
        return Err(ValidationError::Expired);
    }

    if let Some(nbf) = raw.get("nbf").and_then(Value::as_i64) {
        if now + skew < nbf {
            return Err(ValidationError::NotYetValid);
        }
    }

    let subject = raw
        .get("sub")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(ValidationError::MissingSubject)?
        .to_string();

    Ok(OidcClaims {
        issuer: raw
            .get("iss")
            .and_then(Value::as_str)
            .unwrap_or(&config.issuer)
            .to_string(),
        subject,
        email: raw.get("email").and_then(Value::as_str).map(str::to_string),
        // Absent means unverified. Treating a missing claim as "verified"
        // would defeat the check email linking depends on.
        email_verified: raw
            .get("email_verified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        name: raw.get("name").and_then(Value::as_str).map(str::to_string),
        groups: groups_from(&raw, &config.roles_claim),
        raw,
    })
}

/// Read group membership from the configured claim.
///
/// Providers disagree about the shape: an array of strings is usual, but a
/// single string appears too. Both are accepted; anything else yields no
/// groups, which — with deny-by-default — means the user is rejected rather
/// than admitted with a misread membership.
fn groups_from(raw: &HashMap<String, Value>, claim: &str) -> Vec<String> {
    match raw.get(claim) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(single)) => vec![single.clone()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
