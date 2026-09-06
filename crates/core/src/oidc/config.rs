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
/// Where the browser lands after a successful sign-in, when unset.
///
/// A route the admin interface actually has. `/admin` looks plausible and is
/// not one — it would fall through the router's catch-all to the dashboard,
/// which is guarded, and the guard would bounce a freshly signed-in user
/// straight back to the login page.
const DEFAULT_POST_LOGIN_PATH: &str = "/dashboard";
/// How long a resolved identity is reused before being looked up again.
///
/// Short on purpose. This window is how long a user deactivated in
/// `RDataCore` keeps working, so it trades a database round trip per request
/// against the delay before a revocation takes effect.
const DEFAULT_RESOLUTION_CACHE_SECS: u64 = 60;

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
    #[error(
        "RDC_OIDC_CLIENT_ID requires RDC_OIDC_REDIRECT_URI: the browser login flow has \
         nowhere to send the provider's response without it"
    )]
    MissingRedirectUri,
    #[error(
        "RDC_OIDC_POST_LOGIN_PATH must be a path within this application, starting with a \
         single '/', got '{0}'. An absolute or protocol-relative URL here would make the \
         login endpoint an open redirect, which is a phishing primitive."
    )]
    UnsafePostLoginPath(String),
}

/// A configured secret that does not print itself.
///
/// `OidcConfig` derives `Debug`, and configuration gets logged. A bare
/// `String` here would put the client secret in whatever picks that up.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientSecret(String);

impl ClientSecret {
    /// The secret itself. Named so that reaching for it is a visible choice.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ClientSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClientSecret(<redacted>)")
    }
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
    /// How long a resolved identity is reused before re-resolving it.
    pub resolution_cache_ttl: Duration,
    /// This application's client id at the provider. `None` disables the
    /// browser login flow; bearer-token validation needs no client identity.
    pub client_id: Option<String>,
    /// Client secret, where the provider issues one. Absent for a public
    /// client, which PKCE makes safe.
    pub client_secret: Option<ClientSecret>,
    /// Where the provider sends the browser back. Required with `client_id`.
    pub redirect_uri: Option<String>,
    /// Path within this application to land on after a successful sign-in.
    pub post_login_path: String,
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

        let jwks_ttl = duration(map, "RDC_OIDC_JWKS_TTL_SECS", DEFAULT_JWKS_TTL_SECS)?;
        let resolution_cache_ttl = duration(
            map,
            "RDC_OIDC_RESOLUTION_CACHE_SECS",
            DEFAULT_RESOLUTION_CACHE_SECS,
        )?;

        let client_id = get(map, "RDC_OIDC_CLIENT_ID").map(str::to_string);
        let redirect_uri = get(map, "RDC_OIDC_REDIRECT_URI").map(str::to_string);
        // A client id with nowhere to return to is a flow that cannot complete.
        // Failing here beats a 500 on the first sign-in attempt.
        if client_id.is_some() && redirect_uri.is_none() {
            return Err(OidcConfigError::MissingRedirectUri);
        }

        let post_login_path = get(map, "RDC_OIDC_POST_LOGIN_PATH")
            .unwrap_or(DEFAULT_POST_LOGIN_PATH)
            .to_string();
        if !is_local_redirect_path(&post_login_path) {
            return Err(OidcConfigError::UnsafePostLoginPath(post_login_path));
        }

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
            resolution_cache_ttl,
            client_id,
            client_secret: get(map, "RDC_OIDC_CLIENT_SECRET").map(|s| ClientSecret(s.to_string())),
            redirect_uri,
            post_login_path,
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

/// Whether this is a path inside this application, and not a way out of it.
///
/// Used for every value that can become a `Location` header on a sign-in
/// path, so getting it wrong is an open redirect — and on a login endpoint,
/// where the response carries session tokens in its fragment, an open
/// redirect hands those tokens to whoever chose the destination.
///
/// The rules, and why each one is here:
///
/// - It must start with `/`. Anything else is an absolute URL or a relative
///   one, and neither is a place this application controls.
/// - The second character must not be `/` or `\`. `//evil.example.com` is a
///   protocol-relative URL that browsers follow to another origin.
/// - No backslash anywhere. Browsers normalise `\` to `/`, so `/\evil.com`
///   becomes `//evil.com` — the same escape, spelled differently, and the
///   reason a naive `starts_with("//")` check is not enough.
/// - No control characters or whitespace. Browsers strip tabs and newlines
///   from URLs before resolving them, so `/<tab>/evil.com` is another way to
///   arrive at `//evil.com`.
#[must_use]
pub fn is_local_redirect_path(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('/') {
        return false;
    }
    if matches!(chars.next(), Some('/' | '\\')) {
        return false;
    }
    !value
        .chars()
        .any(|c| c == '\\' || c.is_control() || c.is_whitespace())
}

/// Read a duration in seconds, falling back to a default when unset.
fn duration(
    map: &HashMap<String, String>,
    key: &'static str,
    default_secs: u64,
) -> Result<Duration, OidcConfigError> {
    let Some(raw) = get(map, key) else {
        return Ok(Duration::from_secs(default_secs));
    };
    raw.parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|_| OidcConfigError::InvalidNumber(key, raw.to_string()))
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
