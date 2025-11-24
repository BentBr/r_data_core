use crate::refresh_token_repository_trait::RefreshTokenRepositoryTrait;
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_core::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    /// Create a new refresh token repository
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepositoryTrait for RefreshTokenRepository {
    async fn create(
        &self,
        user_id: Uuid,
        token_hash: String,
        expires_at: OffsetDateTime,
        device_info: Option<serde_json::Value>,
    ) -> Result<RefreshToken> {
        let refresh_token = RefreshToken::new(user_id, token_hash, expires_at, device_info);

        let result = sqlx::query_as!(
            RefreshToken,
            "
            INSERT INTO refresh_tokens (
                id, user_id, token_hash, expires_at, created_at, 
                last_used_at, is_revoked, device_info
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING 
                id, user_id, token_hash, expires_at, created_at,
                last_used_at, is_revoked, device_info
            ",
            refresh_token.id,
            refresh_token.user_id,
            refresh_token.token_hash,
            refresh_token.expires_at,
            refresh_token.created_at,
            refresh_token.last_used_at,
            refresh_token.is_revoked,
            refresh_token.device_info,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>> {
        let result = sqlx::query_as!(
            RefreshToken,
            "
            SELECT 
                id, user_id, token_hash, expires_at, created_at,
                last_used_at, is_revoked, device_info
            FROM refresh_tokens 
            WHERE token_hash = $1 AND is_revoked = false
            ",
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result)
    }

    async fn update_last_used(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET last_used_at = NOW() WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }

    async fn revoke_by_id(&self, id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET is_revoked = true WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }

    async fn revoke_by_token_hash(&self, token_hash: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET is_revoked = true WHERE token_hash = $1",
            token_hash
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query!(
            "UPDATE refresh_tokens SET is_revoked = true WHERE user_id = $1 AND is_revoked = false",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result.rows_affected())
    }

    async fn get_active_tokens_for_user(&self, user_id: Uuid) -> Result<Vec<RefreshToken>> {
        let results = sqlx::query_as!(
            RefreshToken,
            "
            SELECT 
                id, user_id, token_hash, expires_at, created_at,
                last_used_at, is_revoked, device_info
            FROM refresh_tokens 
            WHERE user_id = $1 AND is_revoked = false AND expires_at > NOW()
            ORDER BY created_at DESC
            ",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(results)
    }

    async fn cleanup_expired_tokens(&self) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM refresh_tokens WHERE expires_at < NOW() OR is_revoked = true"
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result.rows_affected())
    }

    async fn get_user_token_count(&self, user_id: Uuid) -> Result<i64> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM refresh_tokens WHERE user_id = $1 AND is_revoked = false AND expires_at > NOW()",
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(result.count.unwrap_or(0))
    }

    async fn delete_by_id(&self, id: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM refresh_tokens WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }
}

