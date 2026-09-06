#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{Error, Result};
use crate::identity_repository_trait::IdentityRepositoryTrait;

/// External identity storage backed by `admin_user_identities`.
pub struct IdentityRepository {
    pool: PgPool,
}

impl IdentityRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityRepositoryTrait for IdentityRepository {
    async fn find_user_by_identity(&self, provider: &str, subject: &str) -> Result<Option<Uuid>> {
        sqlx::query_scalar(
            "SELECT admin_user_uuid FROM admin_user_identities \
             WHERE provider = $1 AND subject = $2",
        )
        .bind(provider)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)
    }

    async fn link_identity(
        &self,
        provider: &str,
        subject: &str,
        admin_user_uuid: Uuid,
    ) -> Result<Uuid> {
        // ON CONFLICT DO NOTHING, then read back. Two concurrent first-logins
        // for one subject converge on a single account instead of one of them
        // erroring, and an identity already linked keeps its existing owner —
        // an upsert here would let a second user claim someone else's subject.
        sqlx::query(
            "INSERT INTO admin_user_identities (provider, subject, admin_user_uuid) \
             VALUES ($1, $2, $3) ON CONFLICT (provider, subject) DO NOTHING",
        )
        .bind(provider)
        .bind(subject)
        .bind(admin_user_uuid)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        self.find_user_by_identity(provider, subject)
            .await?
            .ok_or_else(|| Error::Database(sqlx::Error::RowNotFound))
    }

    async fn touch_last_login(&self, provider: &str, subject: &str) -> Result<()> {
        sqlx::query(
            "UPDATE admin_user_identities SET last_login_at = NOW() \
             WHERE provider = $1 AND subject = $2",
        )
        .bind(provider)
        .bind(subject)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(())
    }

    async fn find_local_user_by_email(&self, email: &str) -> Result<Option<Uuid>> {
        // Deliberately excludes SSO-provisioned accounts: linking is for
        // adopting a pre-existing *local* account, and matching another SSO
        // account by address would defeat keying identity on (provider,
        // subject) in the first place.
        sqlx::query_scalar(
            "SELECT uuid FROM admin_users \
             WHERE lower(email) = lower($1) AND is_sso_provisioned = FALSE",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)
    }
}
