#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Turning a verified identity into an `RDataCore` user and its roles.
//!
//! The ordering here is deliberate and worth reading before changing anything:
//! **roles are resolved before any account is created.** An identity that maps
//! to nothing must be refused without leaving a user row behind, and doing the
//! cheap check first is what guarantees that.

use std::sync::Arc;

use rand::Rng as _;
use uuid::Uuid;

use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::error::{Error, Result};
use r_data_core_core::oidc::keys::OidcClaims;
use r_data_core_core::oidc::{map_roles, OidcConfig};
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::admin_user_repository_trait::{
    AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_persistence::{IdentityRepositoryTrait, RoleRepositoryTrait};

/// Length of the throwaway password given to a provisioned account.
const SENTINEL_PASSWORD_BYTES: usize = 48;

/// Resolves verified identities to users and roles.
pub struct OidcProvisioningService {
    identities: Arc<dyn IdentityRepositoryTrait>,
    users: Arc<dyn AdminUserRepositoryTrait>,
    roles: Arc<dyn RoleRepositoryTrait>,
}

impl OidcProvisioningService {
    #[must_use]
    pub const fn new(
        identities: Arc<dyn IdentityRepositoryTrait>,
        users: Arc<dyn AdminUserRepositoryTrait>,
        roles: Arc<dyn RoleRepositoryTrait>,
    ) -> Self {
        Self {
            identities,
            users,
            roles,
        }
    }

    /// Resolve an identity to a user and their roles, creating the user if
    /// this is their first sign-in.
    ///
    /// # Errors
    /// Returns an error when the identity maps to no role and no default is
    /// configured, when the account is not permitted to sign in, or when the
    /// database is unavailable.
    pub async fn resolve_or_provision(
        &self,
        claims: &OidcClaims,
        config: &OidcConfig,
    ) -> Result<(AdminUser, Vec<Role>)> {
        // Roles first. An identity that maps to nothing is refused here,
        // before any account exists, so a rejected sign-in leaves no trace.
        let available = self.available_roles().await?;
        let roles =
            map_roles(claims, config, &available).map_err(|e| Error::Auth(e.to_string()))?;

        let existing = self
            .identities
            .find_user_by_identity(&claims.issuer, &claims.subject)
            .await?;

        let user_uuid = match existing {
            Some(uuid) => uuid,
            None => self.first_sign_in(claims, config).await?,
        };

        self.identities
            .touch_last_login(&claims.issuer, &claims.subject)
            .await?;

        let user = self
            .users
            .find_by_uuid(&user_uuid)
            .await?
            .ok_or_else(|| Error::Auth("the resolved account no longer exists".to_string()))?;

        // The identity provider says who someone is; RDataCore says whether
        // they may act. Without this, deactivating a compromised account here
        // would have no effect for as long as the provider kept issuing
        // tokens — exactly backwards.
        ensure_may_sign_in(&user)?;

        Ok((user, roles))
    }

    /// Adopt or create an account for an identity signing in for the first time.
    async fn first_sign_in(&self, claims: &OidcClaims, config: &OidcConfig) -> Result<Uuid> {
        if let Some(existing) = self.adoptable_local_account(claims, config).await? {
            return self
                .identities
                .link_identity(&claims.issuer, &claims.subject, existing)
                .await;
        }

        let created = self.create_provisioned_user(claims).await?;
        self.identities.mark_sso_provisioned(created).await?;
        self.identities
            .link_identity(&claims.issuer, &claims.subject, created)
            .await
    }

    /// A pre-existing local account this identity may adopt, if any.
    ///
    /// Both conditions are required. The operator must have enabled linking,
    /// *and* the provider must vouch for the address: without the second,
    /// anyone able to set an unverified email at a trusted issuer could claim
    /// someone else's account.
    async fn adoptable_local_account(
        &self,
        claims: &OidcClaims,
        config: &OidcConfig,
    ) -> Result<Option<Uuid>> {
        if !config.link_by_email || !claims.email_verified {
            return Ok(None);
        }
        let Some(email) = claims.email.as_deref() else {
            return Ok(None);
        };
        self.identities.find_local_user_by_email(email).await
    }

    async fn create_provisioned_user(&self, claims: &OidcClaims) -> Result<Uuid> {
        let email = claims
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@sso.invalid", claims.subject));
        let username = self.available_username(&email, claims).await;
        let (first, last) = split_name(claims.name.as_deref());

        self.users
            .create_admin_user(&CreateAdminUserParams {
                username: &username,
                email: &email,
                // Random and immediately discarded. The account must be
                // unusable through any password path, and the flag set by the
                // caller is what enforces that; this only ensures there is no
                // password to guess even if the flag were somehow missed.
                password: &sentinel_password(),
                first_name: &first,
                last_name: &last,
                role: None,
                is_active: true,
                // Self-provisioned: there is no human creator to attribute.
                creator_uuid: Uuid::nil(),
            })
            .await
    }

    /// A username not already taken.
    ///
    /// Usernames are unique, and two people at different providers can easily
    /// share an email local-part. A collision must not fail the sign-in, so a
    /// short suffix is appended until one is free.
    async fn available_username(&self, email: &str, claims: &OidcClaims) -> String {
        let base = email
            .split('@')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&claims.subject)
            .to_string();

        if self.username_free(&base).await {
            return base;
        }
        for _ in 0..5 {
            let candidate = format!("{base}-{}", &Uuid::now_v7().simple().to_string()[..6]);
            if self.username_free(&candidate).await {
                return candidate;
            }
        }
        // Exhausting five random suffixes is implausible; fall back to
        // something guaranteed unique rather than failing the sign-in.
        format!("{base}-{}", Uuid::now_v7().simple())
    }

    async fn username_free(&self, username: &str) -> bool {
        matches!(
            self.users.find_by_username_or_email(username).await,
            Ok(None)
        )
    }

    /// Every external identity linked to one account.
    ///
    /// # Errors
    /// Returns an error if the database is unavailable.
    pub async fn identities_for(&self, admin_user_uuid: Uuid) -> Result<Vec<(String, String)>> {
        self.identities
            .find_identities_for_user(admin_user_uuid)
            .await
    }

    async fn available_roles(&self) -> Result<Vec<Role>> {
        self.roles.list_all(1000, 0, None, None).await
    }
}

/// Refuse an account that exists but may not sign in.
fn ensure_may_sign_in(user: &AdminUser) -> Result<()> {
    if !user.is_active {
        return Err(Error::Auth(
            "this account is deactivated in RDataCore".to_string(),
        ));
    }
    if let Some(until) = user.locked_until {
        if until > time::OffsetDateTime::now_utc() {
            return Err(Error::Auth("this account is locked".to_string()));
        }
    }
    Ok(())
}

/// A password nothing can match, generated and immediately forgotten.
fn sentinel_password() -> String {
    let mut rng = rand::rng();
    (0..SENTINEL_PASSWORD_BYTES)
        .map(|_| char::from(rng.random_range(b'!'..=b'~')))
        .collect()
}

/// Split a display name into first and last, tolerating any shape.
fn split_name(name: Option<&str>) -> (String, String) {
    let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
        return (String::new(), String::new());
    };
    name.split_once(' ').map_or_else(
        || (name.to_string(), String::new()),
        |(first, rest)| (first.to_string(), rest.trim().to_string()),
    )
}

#[cfg(test)]
mod oidc_provisioning_tests;
