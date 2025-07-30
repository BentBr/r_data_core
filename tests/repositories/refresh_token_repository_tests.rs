use crate::common::utils;
use r_data_core::{
    entity::refresh_token::{RefreshToken, RefreshTokenRepository, RefreshTokenRepositoryTrait},
    error::Result,
};
use serial_test::serial;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_create_refresh_token() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Test data
    let token_hash = RefreshToken::hash_token("test_token")?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
    let device_info = Some(serde_json::json!({
        "user_agent": "test-agent",
        "device": "test-device"
    }));

    // Create refresh token
    let created_token = repo
        .create(
            user_uuid,
            token_hash.clone(),
            expires_at,
            device_info.clone(),
        )
        .await?;

    // Verify
    assert_eq!(created_token.user_id, user_uuid);
    assert_eq!(created_token.token_hash, token_hash);
    assert_eq!(created_token.expires_at, expires_at);
    assert_eq!(created_token.device_info, device_info);
    assert!(!created_token.is_revoked);
    assert!(created_token.last_used_at.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_refresh_token_by_hash() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Create a token
    let token_hash = RefreshToken::hash_token("find_test_token")?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let created_token = repo
        .create(user_uuid, token_hash.clone(), expires_at, None)
        .await?;

    // Find the token
    let found_token = repo.find_by_token_hash(&token_hash).await?;

    // Verify
    assert!(found_token.is_some());
    let found = found_token.unwrap();
    assert_eq!(found.id, created_token.id);
    assert_eq!(found.user_id, user_uuid);
    assert_eq!(found.token_hash, token_hash);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_nonexistent_token() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let repo = RefreshTokenRepository::new(pool.clone());

    // Try to find non-existent token
    let fake_hash = RefreshToken::hash_token("nonexistent_token")?;
    let result = repo.find_by_token_hash(&fake_hash).await?;

    // Verify
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_last_used() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Create a token
    let token_hash = RefreshToken::hash_token("last_used_test")?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let created_token = repo
        .create(user_uuid, token_hash.clone(), expires_at, None)
        .await?;

    // Verify initial state
    assert!(created_token.last_used_at.is_none());

    // Update last used
    repo.update_last_used(created_token.id).await?;

    // Verify updated
    let updated_token = repo.find_by_token_hash(&token_hash).await?;
    assert!(updated_token.is_some());
    assert!(updated_token.unwrap().last_used_at.is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_revoke_token_by_id() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Create a token
    let token_hash = RefreshToken::hash_token("revoke_test")?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let created_token = repo
        .create(user_uuid, token_hash.clone(), expires_at, None)
        .await?;

    // Verify initial state
    assert!(!created_token.is_revoked);

    // Revoke token
    repo.revoke_by_id(created_token.id).await?;

    // Verify revoked
    let revoked_token = repo.find_by_token_hash(&token_hash).await?;
    assert!(revoked_token.is_none()); // Should not find revoked tokens

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_revoke_token_by_hash() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Create a token
    let token_hash = RefreshToken::hash_token("revoke_hash_test")?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    repo.create(user_uuid, token_hash.clone(), expires_at, None)
        .await?;

    // Revoke by hash
    repo.revoke_by_token_hash(&token_hash).await?;

    // Verify revoked
    let revoked_token = repo.find_by_token_hash(&token_hash).await?;
    assert!(revoked_token.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_revoke_all_user_tokens() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_refresh_tokens(&pool).await?;

    let user_uuid = utils::create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.clone());

    // Create multiple tokens for the user
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let token1_hash = RefreshToken::hash_token("user_token_1")?;
    let token2_hash = RefreshToken::hash_token("user_token_2")?;

    repo.create(user_uuid, token1_hash.clone(), expires_at, None)
        .await?;
    repo.create(user_uuid, token2_hash.clone(), expires_at, None)
        .await?;

    // Revoke all tokens for user
    let revoked_count = repo.revoke_all_for_user(user_uuid).await?;

    // Verify
    assert_eq!(revoked_count, 2);

    // Check that tokens are no longer findable
    assert!(repo.find_by_token_hash(&token1_hash).await?.is_none());
    assert!(repo.find_by_token_hash(&token2_hash).await?.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_expiration_validation() -> Result<()> {
    // Create token instance
    let user_uuid = Uuid::now_v7();
    let token_hash = "test_hash".to_string();
    let device_info = None;

    // Test expired token
    let expired_time = OffsetDateTime::now_utc() - Duration::hours(1);
    let expired_token = RefreshToken::new(
        user_uuid,
        token_hash.clone(),
        expired_time,
        device_info.clone(),
    );
    assert!(expired_token.is_expired());
    assert!(!expired_token.is_valid());

    // Test valid token
    let valid_time = OffsetDateTime::now_utc() + Duration::hours(1);
    let valid_token = RefreshToken::new(user_uuid, token_hash, valid_time, device_info);
    assert!(!valid_token.is_expired());
    assert!(valid_token.is_valid());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_generation_and_hashing() -> Result<()> {
    // Generate token
    let token1 = RefreshToken::generate_token();
    let token2 = RefreshToken::generate_token();

    // Verify tokens are different
    assert_ne!(token1, token2);
    assert!(!token1.is_empty());
    assert!(!token2.is_empty());

    // Test hashing
    let hash1 = RefreshToken::hash_token(&token1)?;
    let hash2 = RefreshToken::hash_token(&token1)?; // Same token
    let hash3 = RefreshToken::hash_token(&token2)?; // Different token

    // Same token should produce same hash
    assert_eq!(hash1, hash2);
    // Different tokens should produce different hashes
    assert_ne!(hash1, hash3);

    Ok(())
}
