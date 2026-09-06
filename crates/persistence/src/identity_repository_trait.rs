#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use crate::core::error::Result;

/// External identities linked to admin users.
///
/// Identity is keyed on `(provider, subject)` throughout — the issuer URL and
/// the provider's `sub` claim. Never on email: it is mutable and re-assignable
/// at most providers, so keying on it would let anyone able to register an
/// address at a trusted issuer inherit the matching account.
#[async_trait]
pub trait IdentityRepositoryTrait: Send + Sync {
    /// The admin user this external identity belongs to, if it is known.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    async fn find_user_by_identity(&self, provider: &str, subject: &str) -> Result<Option<Uuid>>;

    /// Attach an external identity to an admin user, idempotently.
    ///
    /// Returns the user the identity resolves to, which may not be the one
    /// passed in: two concurrent first-logins for the same subject must
    /// converge on one account rather than racing, and an identity already
    /// linked elsewhere keeps its existing owner. Last-writer-wins here would
    /// be an account-takeover primitive.
    ///
    /// # Errors
    /// Returns an error if the database write fails.
    async fn link_identity(
        &self,
        provider: &str,
        subject: &str,
        admin_user_uuid: Uuid,
    ) -> Result<Uuid>;

    /// Record that this identity just signed in.
    ///
    /// # Errors
    /// Returns an error if the database write fails.
    async fn touch_last_login(&self, provider: &str, subject: &str) -> Result<()>;

    /// Mark an account as created through SSO.
    ///
    /// Separate from creation because `create_admin_user` is shared with the
    /// local-account path and widening its parameters would touch every
    /// caller. The window between the two statements is safe: the account's
    /// password is random and discarded, so no password path can use it even
    /// before the flag lands.
    ///
    /// # Errors
    /// Returns an error if the database write fails.
    async fn mark_sso_provisioned(&self, admin_user_uuid: Uuid) -> Result<()>;

    /// A local account with this address, for optional email linking.
    ///
    /// Only ever called when the operator has enabled linking *and* the
    /// provider vouched for the address; the repository does not decide that.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    async fn find_local_user_by_email(&self, email: &str) -> Result<Option<Uuid>>;

    /// Every external identity linked to one account, as `(provider, subject)`.
    ///
    /// Used to evict cached authorization when the account changes. The cache
    /// is keyed on the identity rather than the account, so a change made
    /// inside `RDataCore` has no way to find the entries it invalidates
    /// without this.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    async fn find_identities_for_user(
        &self,
        admin_user_uuid: Uuid,
    ) -> Result<Vec<(String, String)>>;
}
