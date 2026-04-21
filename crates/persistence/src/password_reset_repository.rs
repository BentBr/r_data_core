#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::password_reset_repository_trait::PasswordResetRepositoryTrait;
use r_data_core_core::error::Result;
use r_data_core_core::password_reset_token::PasswordResetToken;

/// Repository for password reset token operations
pub struct PasswordResetRepository {
    pool: PgPool,
}

impl PasswordResetRepository {
    /// Create a new password reset repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PasswordResetRepositoryTrait for PasswordResetRepository {
    async fn insert_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            user_id,
            token_hash,
            expires_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(id)
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<PasswordResetToken>> {
        let row = sqlx::query_as!(
            PasswordResetToken,
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, used_at
            FROM password_reset_tokens
            WHERE token_hash = $1
            "#,
            hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(row)
    }

    async fn find_latest_for_user(&self, user_id: Uuid) -> Result<Option<PasswordResetToken>> {
        let row = sqlx::query_as!(
            PasswordResetToken,
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at, used_at
            FROM password_reset_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(row)
    }

    async fn mark_used(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64> {
        let result = sqlx::query!("DELETE FROM password_reset_tokens WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result.rows_affected())
    }

    async fn delete_for_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }
}
