#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! OIDC configuration, and what it refuses.
//!
//! Two defaults here are load-bearing and deliberately inconvenient. An
//! unmapped user is **rejected**, not admitted with no permissions. And
//! linking an SSO identity to an existing local account by email address is
//! **off**, because it is an account-takeover surface the moment an operator
//! points at an issuer with unverified addresses.

use std::collections::HashMap;
use std::time::Duration;

use thiserror::Error;

/// Claim to read group membership from when unset.
const DEFAULT_ROLES_CLAIM: &str = "groups";
/// How long a fetched key set stays usable when unset.
const DEFAULT_JWKS_TTL_SECS: u64 = 3600;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OidcConfigError {
    #[error(
        "RDC_OIDC_ISSUER requires RDC_OIDC_AUDIENCE: without it, any token from that \
         issuer would be accepted, including ones minted for a different service"
    )]
    MissingAudience,
    #[error(
        "RDC_OIDC_ROLE_MAP entry '{0}' is not 'idp-group:rdc-role'. A silently ignored \
         mapping produces a role map that grants nothing, with no indication why."
    )]
    MalformedRoleMap(String),
    #[error("{0} must be a positive integer, got '{1}'")]
    InvalidNumber(&'static str, String),
}

/// How this instance trusts an external identity provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    /// Claim holding the user's groups, e.g. `groups`.
    pub roles_claim: String,
    /// `IdP` group name → `RDataCore` role name.
    pub role_map: HashMap<String, String>,
    /// Role for a user matching no mapping. `None` means reject them.
    pub default_role: Option<String>,
    /// Whether a first login may attach to an existing local account with the
    /// same address. Requires `email_verified` on the token even when enabled.
    pub link_by_email: bool,
    pub jwks_ttl: Duration,
}

impl OidcConfig {
    /// Read OIDC settings, or `None` when the feature is not configured.
    ///
    /// Presence of `RDC_OIDC_ISSUER` is what enables OIDC. Everything else is
    /// either defaulted or, where a wrong default would be unsafe, required.
    ///
    /// # Errors
    /// Returns `OidcConfigError` when the issuer is set but the configuration
    /// around it is incomplete or malformed.
    pub fn from_map(map: &HashMap<String, String>) -> Result<Option<Self>, OidcConfigError> {
        let Some(issuer) = get(map, "RDC_OIDC_ISSUER") else {
            return Ok(None);
        };

        let audience = get(map, "RDC_OIDC_AUDIENCE").ok_or(OidcConfigError::MissingAudience)?;

        let jwks_ttl = match get(map, "RDC_OIDC_JWKS_TTL_SECS") {
            None => Duration::from_secs(DEFAULT_JWKS_TTL_SECS),
            Some(raw) => raw.parse::<u64>().map_or_else(
                |_| {
                    Err(OidcConfigError::InvalidNumber(
                        "RDC_OIDC_JWKS_TTL_SECS",
                        raw.to_string(),
                    ))
                },
                |secs| Ok(Duration::from_secs(secs)),
            )?,
        };

        Ok(Some(Self {
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            roles_claim: get(map, "RDC_OIDC_ROLES_CLAIM")
                .unwrap_or(DEFAULT_ROLES_CLAIM)
                .to_string(),
            role_map: parse_role_map(get(map, "RDC_OIDC_ROLE_MAP"))?,
            // Absent means reject, never "admit with nothing".
            default_role: get(map, "RDC_OIDC_DEFAULT_ROLE").map(str::to_string),
            link_by_email: get(map, "RDC_OIDC_LINK_BY_EMAIL")
                .is_some_and(|v| v.eq_ignore_ascii_case("true")),
            jwks_ttl,
        }))
    }

    /// Read from the process environment.
    ///
    /// # Errors
    /// As [`Self::from_map`].
    pub fn from_env() -> Result<Option<Self>, OidcConfigError> {
        Self::from_map(&std::env::vars().collect())
    }
}

/// Parse `idp-group:rdc-role,other-group:other-role`.
///
/// A malformed entry is an error rather than a skip. Silently dropping one
/// leaves an operator with a role map that grants nothing and no clue why —
/// and the symptom, everybody being rejected, looks like a different problem.
fn parse_role_map(raw: Option<&str>) -> Result<HashMap<String, String>, OidcConfigError> {
    let Some(raw) = raw else {
        return Ok(HashMap::new());
    };
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .split_once(':')
                .filter(|(group, role)| !group.trim().is_empty() && !role.trim().is_empty())
                .map(|(group, role)| (group.trim().to_string(), role.trim().to_string()))
                .ok_or_else(|| OidcConfigError::MalformedRoleMap(entry.to_string()))
        })
        .collect()
}

/// Fetch a variable, treating an empty value as absent.
fn get<'a>(map: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    map.get(key).map(String::as_str).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests;
