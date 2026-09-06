#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Where signing keys come from, without saying how they are fetched.
//!
//! `core` has no HTTP dependency and must not gain one: it is the bottom of
//! the dependency graph, and a domain leaf that makes network calls is a
//! different kind of thing. So validation takes keys through this trait, and
//! the implementation that actually talks to an identity provider lives in
//! `services`, which already has `reqwest`.
//!
//! It also makes validation testable with a fixed key set, which matters —
//! the tests that count here are the rejections, and they should not need a
//! server.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeySourceError {
    #[error("could not reach the identity provider: {0}")]
    Unreachable(String),
    #[error("the identity provider's response was not a usable key set: {0}")]
    Malformed(String),
    #[error("no key with id '{0}'")]
    UnknownKid(String),
}

/// One signing key, in the subset of JWK this needs.
///
/// Deliberately not the whole specification: only RSA and EC keys are
/// accepted, and only what verifying a signature requires is modelled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwk {
    /// Key id, matched against the token header's `kid`.
    pub kid: String,
    /// Key type: `RSA` or `EC`.
    pub kty: String,
    /// Algorithm, where the provider states one.
    #[serde(default)]
    pub alg: Option<String>,
    /// RSA modulus.
    #[serde(default)]
    pub n: Option<String>,
    /// RSA exponent.
    #[serde(default)]
    pub e: Option<String>,
    /// EC curve.
    #[serde(default)]
    pub crv: Option<String>,
    /// EC x coordinate.
    #[serde(default)]
    pub x: Option<String>,
    /// EC y coordinate.
    #[serde(default)]
    pub y: Option<String>,
}

/// A provider's published signing keys.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwkSet {
    #[serde(default)]
    pub keys: Vec<Jwk>,
}

impl JwkSet {
    /// The key with this id, if the set has one.
    #[must_use]
    pub fn find(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|k| k.kid == kid)
    }
}

/// Supplies the signing keys a token is verified against.
#[async_trait::async_trait]
pub trait KeySource: Send + Sync {
    /// The current key set, cached or freshly fetched.
    ///
    /// # Errors
    /// Returns `KeySourceError` when the provider cannot be reached or its
    /// response cannot be parsed.
    async fn keys(&self) -> Result<JwkSet, KeySourceError>;

    /// Re-fetch because a token named a key id the cached set does not have.
    ///
    /// Providers rotate keys, so an unknown id is expected occasionally and is
    /// not by itself an error. An implementation must rate-limit this: an
    /// attacker sending tokens with random ids would otherwise turn this
    /// server into a load generator aimed at the identity provider.
    ///
    /// # Errors
    /// As [`Self::keys`].
    async fn refresh_for_unknown_kid(&self, kid: &str) -> Result<JwkSet, KeySourceError>;
}

/// A key source backed by a fixed set. For tests and for offline validation.
pub struct StaticKeySource {
    keys: JwkSet,
}

impl StaticKeySource {
    #[must_use]
    pub const fn new(keys: JwkSet) -> Self {
        Self { keys }
    }
}

#[async_trait::async_trait]
impl KeySource for StaticKeySource {
    async fn keys(&self) -> Result<JwkSet, KeySourceError> {
        Ok(self.keys.clone())
    }

    async fn refresh_for_unknown_kid(&self, kid: &str) -> Result<JwkSet, KeySourceError> {
        // Nothing to refresh from; say so rather than returning the same set
        // and letting the caller loop.
        Err(KeySourceError::UnknownKid(kid.to_string()))
    }
}

/// Claims this system reads from an identity token.
///
/// Everything else the provider sends is ignored. A claim that is not read
/// cannot influence an authorization decision, which is the point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcClaims {
    pub issuer: String,
    /// The provider's stable identifier for this person. Identity is keyed on
    /// `(issuer, subject)` and never on email, which is mutable and
    /// re-assignable at most providers.
    pub subject: String,
    pub email: Option<String>,
    /// Whether the provider vouches for the address. Email linking requires
    /// this even when the operator has enabled it.
    pub email_verified: bool,
    pub name: Option<String>,
    /// Group membership, read from the configured claim.
    pub groups: Vec<String>,
    /// Every other top-level claim, kept for diagnostics only.
    pub raw: HashMap<String, serde_json::Value>,
}
