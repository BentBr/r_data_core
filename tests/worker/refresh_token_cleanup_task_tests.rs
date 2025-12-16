#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::error::Result;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};
use r_data_core_test_support::{clear_refresh_tokens, create_test_admin_user, setup_test_db};
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::refresh_token_cleanup::RefreshTokenCleanupTask;
use serial_test::serial;
use time::{Duration, OffsetDateTime};

#[tokio::test]
#[serial]
async fn test_refresh_token_cleanup_task_removes_expired_tokens() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    clear_refresh_tokens(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.pool.clone());

    // Create tokens: one expired, one valid
    let expired_token_hash = RefreshToken::hash_token("expired_token")?;
    let valid_token_hash = RefreshToken::hash_token("valid_token")?;

    let expired_expires_at = OffsetDateTime::now_utc() - Duration::days(1);
    let valid_expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let expired_token = repo
        .create(
            user_uuid,
            expired_token_hash.clone(),
            expired_expires_at,
            None,
        )
        .await?;

    let _valid_token = repo
        .create(user_uuid, valid_token_hash.clone(), valid_expires_at, None)
        .await?;

    // Manually expire the first token to ensure it's expired
    sqlx::query!(
        "UPDATE refresh_tokens SET expires_at = NOW() - INTERVAL '1 day' WHERE id = $1",
        expired_token.id
    )
    .execute(&pool.pool)
    .await?;

    // Verify tokens exist before cleanup
    assert!(
        repo.find_by_token_hash(&expired_token_hash)
            .await?
            .is_some(),
        "Expired token should exist before cleanup"
    );
    assert!(
        repo.find_by_token_hash(&valid_token_hash).await?.is_some(),
        "Valid token should exist before cleanup"
    );

    // Execute the cleanup task
    let task = RefreshTokenCleanupTask::new("0 0 * * * *".to_string());
    let context = TaskContext::new(pool.pool.clone());
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify expired token is removed
    assert!(
        repo.find_by_token_hash(&expired_token_hash)
            .await?
            .is_none(),
        "Expired token should be cleaned up"
    );

    // Verify valid token still exists
    assert!(
        repo.find_by_token_hash(&valid_token_hash).await?.is_some(),
        "Valid token should still exist after cleanup"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_refresh_token_cleanup_task_removes_revoked_tokens() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    clear_refresh_tokens(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.pool.clone());

    // Create tokens: one revoked, one active
    let revoked_token_hash = RefreshToken::hash_token("revoked_token")?;
    let active_token_hash = RefreshToken::hash_token("active_token")?;

    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let revoked_token = repo
        .create(user_uuid, revoked_token_hash.clone(), expires_at, None)
        .await?;

    let _active_token = repo
        .create(user_uuid, active_token_hash.clone(), expires_at, None)
        .await?;

    // Revoke the first token
    repo.revoke_by_id(revoked_token.id).await?;

    // Verify tokens exist before cleanup (revoked token won't be found by find_by_token_hash,
    // but it still exists in the database)
    let revoked_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM refresh_tokens WHERE id = $1 AND is_revoked = true",
        revoked_token.id
    )
    .fetch_one(&pool.pool)
    .await?;
    assert_eq!(
        revoked_count.count.unwrap_or(0),
        1,
        "Revoked token should exist in database"
    );
    assert!(
        repo.find_by_token_hash(&active_token_hash).await?.is_some(),
        "Active token should exist before cleanup"
    );

    // Execute the cleanup task
    let task = RefreshTokenCleanupTask::new("0 0 * * * *".to_string());
    let context = TaskContext::new(pool.pool.clone());
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify revoked token is removed
    let revoked_count_after = sqlx::query!(
        "SELECT COUNT(*) as count FROM refresh_tokens WHERE id = $1",
        revoked_token.id
    )
    .fetch_one(&pool.pool)
    .await?;
    assert_eq!(
        revoked_count_after.count.unwrap_or(0),
        0,
        "Revoked token should be cleaned up"
    );

    // Verify active token still exists
    assert!(
        repo.find_by_token_hash(&active_token_hash).await?.is_some(),
        "Active token should still exist after cleanup"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_refresh_token_cleanup_task_removes_both_expired_and_revoked() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    clear_refresh_tokens(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = RefreshTokenRepository::new(pool.pool.clone());

    // Create tokens: one expired, one revoked, one valid
    let expired_token_hash = RefreshToken::hash_token("expired_token")?;
    let revoked_token_hash = RefreshToken::hash_token("revoked_token")?;
    let valid_token_hash = RefreshToken::hash_token("valid_token")?;

    let expired_expires_at = OffsetDateTime::now_utc() - Duration::days(1);
    let valid_expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let expired_token = repo
        .create(
            user_uuid,
            expired_token_hash.clone(),
            expired_expires_at,
            None,
        )
        .await?;

    let revoked_token = repo
        .create(
            user_uuid,
            revoked_token_hash.clone(),
            valid_expires_at,
            None,
        )
        .await?;

    let _valid_token = repo
        .create(user_uuid, valid_token_hash.clone(), valid_expires_at, None)
        .await?;

    // Manually expire the first token
    sqlx::query!(
        "UPDATE refresh_tokens SET expires_at = NOW() - INTERVAL '1 day' WHERE id = $1",
        expired_token.id
    )
    .execute(&pool.pool)
    .await?;

    // Revoke the second token
    repo.revoke_by_id(revoked_token.id).await?;

    // Execute the cleanup task
    let task = RefreshTokenCleanupTask::new("0 0 * * * *".to_string());
    let context = TaskContext::new(pool.pool.clone());
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify expired token is removed
    assert!(
        repo.find_by_token_hash(&expired_token_hash)
            .await?
            .is_none(),
        "Expired token should be cleaned up"
    );

    // Verify revoked token is removed
    let revoked_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM refresh_tokens WHERE id = $1",
        revoked_token.id
    )
    .fetch_one(&pool.pool)
    .await?;
    assert_eq!(
        revoked_count.count.unwrap_or(0),
        0,
        "Revoked token should be cleaned up"
    );

    // Verify valid token still exists
    assert!(
        repo.find_by_token_hash(&valid_token_hash).await?.is_some(),
        "Valid token should still exist after cleanup"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_refresh_token_cleanup_task_name_and_cron() -> Result<()> {
    let task = RefreshTokenCleanupTask::new("0 0 * * * *".to_string());

    assert_eq!(task.name(), "refresh_token_cleanup");
    assert_eq!(task.cron(), "0 0 * * * *");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_refresh_token_cleanup_task_with_no_tokens() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    clear_refresh_tokens(&pool).await?;

    // Execute the cleanup task with no tokens
    let task = RefreshTokenCleanupTask::new("0 0 * * * *".to_string());
    let context = TaskContext::new(pool.pool.clone());

    // Should not error when there are no tokens to clean
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    Ok(())
}
