#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Who the server acts for, and what it is allowed to do.
//!
//! The invariant this module exists to hold: **the MCP server never acts as
//! anyone but the caller.** There is no service account and no elevated
//! fallback. Every outbound request carries a credential belonging to the
//! person who made the call, so a 403 from `RDataCore` is the authoritative
//! answer and no bug here can grant more than the human has.

pub mod api_key;

pub use api_key::ApiKeyBackend;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no credential available for this caller")]
    MissingCredential,
    #[error("token rejected: {0}")]
    InvalidToken(String),
    #[error("could not load permissions: {0}")]
    PermissionLookup(String),
}

/// Per-request caller identity.
///
/// In stdio mode there is exactly one caller and this stays empty. In HTTP mode
/// the transport validates the inbound bearer once per request and records the
/// result here, so every tool in that request shares one validation.
///
/// Threaded through every tool from the start even though stdio never
/// populates it: retrofitting it later would mean touching every tool
/// signature, which is precisely what the production-auth plan must not have
/// to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallerContext {
    pub bearer: Option<String>,
    /// Set by the HTTP layer after successful validation. A backend must refuse
    /// to produce headers for an unvalidated context — defence in depth, so a
    /// future transport that forgets to validate fails closed.
    pub validated: bool,
}

/// What the caller may do, as reported by `RDataCore`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Permissions {
    pub is_super_admin: bool,
    pub entries: Vec<String>,
}

impl Permissions {
    /// Whether the caller holds `needed`, e.g. `"workflows:write"`.
    ///
    /// Permission strings come in two shapes — `namespace:action` and
    /// `namespace:path:action` (see `AuthUserClaims.permissions`). Someone
    /// holding only `entities:/customers:read` genuinely can read entities,
    /// just not all of them, so an exact match would wrongly hide the query
    /// tool from them. Matching therefore compares namespace and action and
    /// ignores any interposed path.
    ///
    /// Being generous is the right bias: this gates what is *advertised*,
    /// never what is *allowed*. `RDataCore` checks the specific path on every
    /// call and answers 403 if it is out of scope, which the model reads as an
    /// ordinary teaching error.
    #[must_use]
    pub fn allows(&self, needed: &str) -> bool {
        if self.is_super_admin {
            return true;
        }
        let Some((want_namespace, want_action)) = needed.split_once(':') else {
            return false;
        };
        self.entries.iter().any(|held| {
            held.split_once(':').is_some_and(|(namespace, rest)| {
                // `rest` is either "action" or "path:action".
                let action = rest.rsplit_once(':').map_or(rest, |(_, action)| action);
                namespace == want_namespace && action == want_action
            })
        })
    }
}

/// Supplies outbound credentials for a caller.
///
/// Async because the OIDC backend added by the production-auth plan exchanges
/// the caller's identity for a short-lived local token rather than forwarding
/// anything. Making it async now costs nothing and avoids reopening every
/// implementation later.
///
/// There is deliberately no `permissions` method here: that is an ordinary
/// client call made with whatever credential the backend produced, so putting
/// it on this trait would only duplicate it.
#[async_trait::async_trait]
pub trait AuthBackend: Send + Sync {
    /// Headers to attach to the outbound `RDataCore` request.
    ///
    /// # Errors
    /// Returns `AuthError` when no usable credential can be obtained for this
    /// caller. Never falls back to a credential belonging to anyone else.
    async fn outbound_headers(
        &self,
        ctx: &CallerContext,
    ) -> Result<Vec<(String, String)>, AuthError>;
}

#[cfg(test)]
mod tests;
