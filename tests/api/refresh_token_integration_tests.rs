use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};
use r_data_core_core::error::Result;
use serial_test::serial;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::utils;

    #[tokio::test]
    #[serial]
    async fn test_refresh_token_creation_and_validation() -> Result<()> {
        // Setup test database with proper cleaning
        let pool = utils::setup_test_db().await;
        utils::clear_test_db(&pool).await?;
        utils::clear_refresh_tokens(&pool).await?;

        // Create a repository to work with refresh tokens directly
        let repo = RefreshTokenRepository::new(pool.clone());

        // Create a test admin user
        let user_uuid = utils::create_test_admin_user(&pool).await?;

        // Create a test refresh token
        let token_hash = "test_token_hash_123";
        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(30);
        let refresh_token = repo
            .create(user_uuid, token_hash.to_string(), expires_at, None)
            .await?;

        // Verify the token was created correctly
        assert_eq!(
            refresh_token.user_id, user_uuid,
            "Token should belong to correct user"
        );
        assert_eq!(
            refresh_token.token_hash, token_hash,
            "Token hash should match"
        );
        assert!(!refresh_token.is_revoked, "New token should not be revoked");

        // Find the token by hash
        let found_token = repo.find_by_token_hash(token_hash).await?;
        assert!(found_token.is_some(), "Token should be found by hash");
        let found_token = found_token.unwrap();
        assert_eq!(
            found_token.id, refresh_token.id,
            "Found token should match created token"
        );

        // Update last used timestamp
        repo.update_last_used(refresh_token.id).await?;

        // Verify the token is still valid
        let updated_token = repo.find_by_token_hash(token_hash).await?;
        assert!(
            updated_token.is_some(),
            "Token should still be valid after update"
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_token_expiration() -> Result<()> {
        // Setup test database with proper cleaning
        let pool = utils::setup_test_db().await;
        utils::clear_test_db(&pool).await?;
        utils::clear_refresh_tokens(&pool).await?;

        // Create a repository to work with refresh tokens directly
        let repo = RefreshTokenRepository::new(pool.clone());

        // Create a test admin user
        let user_uuid = utils::create_test_admin_user(&pool).await?;

        // Create a test refresh token with short expiration (1 day)
        let token_hash = "expired_token_hash";
        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(1);
        let refresh_token = repo
            .create(user_uuid, token_hash.to_string(), expires_at, None)
            .await?;

        // Manually expire the token by setting expires_at to the past
        sqlx::query!(
            "UPDATE refresh_tokens SET expires_at = NOW() - INTERVAL '1 day' WHERE id = $1",
            refresh_token.id
        )
        .execute(&pool)
        .await?;

        // Try to find the expired token - it should still be found but marked as expired
        let expired_token = repo.find_by_token_hash(token_hash).await?;
        // Note: The repository doesn't filter by expiration, so we need to check manually
        if let Some(token) = expired_token {
            assert!(
                token.expires_at < OffsetDateTime::now_utc(),
                "Token should be expired"
            );
        }

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_token_revocation() -> Result<()> {
        // Setup test database with proper cleaning
        let pool = utils::setup_test_db().await;
        utils::clear_test_db(&pool).await?;
        utils::clear_refresh_tokens(&pool).await?;

        // Create a repository to work with refresh tokens directly
        let repo = RefreshTokenRepository::new(pool.clone());

        // Create a test admin user
        let user_uuid = utils::create_test_admin_user(&pool).await?;

        // Create a test refresh token
        let token_hash = "revoked_token_hash";
        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(30);
        let refresh_token = repo
            .create(user_uuid, token_hash.to_string(), expires_at, None)
            .await?;

        // Verify the token is active initially
        let active_token = repo.find_by_token_hash(token_hash).await?;
        assert!(active_token.is_some(), "Token should be active initially");

        // Revoke the token
        repo.revoke_by_id(refresh_token.id).await?;

        // Try to find the revoked token - it should not be found (filtered out)
        let revoked_token = repo.find_by_token_hash(token_hash).await?;
        assert!(revoked_token.is_none(), "Revoked token should not be found");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_token_cleanup() -> Result<()> {
        // Setup test database with proper cleaning
        let pool = utils::setup_test_db().await;
        utils::clear_test_db(&pool).await?;
        utils::clear_refresh_tokens(&pool).await?;

        // Create a repository to work with refresh tokens directly
        let repo = RefreshTokenRepository::new(pool.clone());

        // Create a test admin user
        let user_uuid = utils::create_test_admin_user(&pool).await?;

        // Create multiple tokens
        let token1_hash = "token1_hash";
        let token2_hash = "token2_hash";
        let expires_at1 = OffsetDateTime::now_utc() + time::Duration::days(1);
        let expires_at2 = OffsetDateTime::now_utc() + time::Duration::days(30);

        let token1 = repo
            .create(user_uuid, token1_hash.to_string(), expires_at1, None)
            .await?;
        let token2 = repo
            .create(user_uuid, token2_hash.to_string(), expires_at2, None)
            .await?;

        // Manually expire the first token
        sqlx::query!(
            "UPDATE refresh_tokens SET expires_at = NOW() - INTERVAL '1 day' WHERE id = $1",
            token1.id
        )
        .execute(&pool)
        .await?;

        // Cleanup expired tokens
        let cleaned_count = repo.cleanup_expired_tokens().await?;
        assert_eq!(cleaned_count, 1, "Should clean up 1 expired token");

        // Verify the expired token is gone
        let expired_token = repo.find_by_token_hash(token1_hash).await?;
        assert!(
            expired_token.is_none(),
            "Expired token should be cleaned up"
        );

        // Verify the valid token still exists
        let valid_token = repo.find_by_token_hash(token2_hash).await?;
        assert!(valid_token.is_some(), "Valid token should still exist");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_refresh_token_user_association() -> Result<()> {
        // Setup test database with proper cleaning
        let pool = utils::setup_test_db().await;
        utils::clear_test_db(&pool).await?;
        utils::clear_refresh_tokens(&pool).await?;

        // Create a repository to work with refresh tokens directly
        let repo = RefreshTokenRepository::new(pool.clone());

        // Create test admin users
        let user1_uuid = utils::create_test_admin_user(&pool).await?;
        let user2_uuid = utils::create_test_admin_user(&pool).await?;

        // Create tokens for both users
        let token1_hash = "user1_token_hash";
        let token2_hash = "user2_token_hash";
        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(30);

        let token1 = repo
            .create(user1_uuid, token1_hash.to_string(), expires_at, None)
            .await?;
        let token2 = repo
            .create(user2_uuid, token2_hash.to_string(), expires_at, None)
            .await?;

        // Verify tokens are associated with correct users
        assert_eq!(token1.user_id, user1_uuid, "Token1 should belong to user1");
        assert_eq!(token2.user_id, user2_uuid, "Token2 should belong to user2");

        // Get active tokens for each user
        let user1_tokens = repo.get_active_tokens_for_user(user1_uuid).await?;
        let user2_tokens = repo.get_active_tokens_for_user(user2_uuid).await?;

        assert_eq!(user1_tokens.len(), 1, "User1 should have 1 active token");
        assert_eq!(user2_tokens.len(), 1, "User2 should have 1 active token");

        Ok(())
    }
}
