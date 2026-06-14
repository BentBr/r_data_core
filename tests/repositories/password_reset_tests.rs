#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::{PasswordResetRepository, PasswordResetRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use time::Duration;

#[tokio::test]
#[serial]
async fn test_password_reset_token_lifecycle() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    // Create a test user first (password_reset_tokens has FK to admin_users)
    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = PasswordResetRepository::new(pool.pool.clone());

    // Insert token
    let token_hash = "abc123hash_lifecycle_test";
    let expires_at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    let id = repo.insert_token(user_uuid, token_hash, expires_at).await?;

    // Find by hash
    let found = repo
        .find_by_token_hash(token_hash)
        .await?
        .expect("token should be findable by hash");
    assert_eq!(found.id, id);
    assert_eq!(found.user_id, user_uuid);
    assert_eq!(found.token_hash, token_hash);
    assert!(
        found.used_at.is_none(),
        "fresh token should not be marked used"
    );

    // Find latest for user
    let latest = repo
        .find_latest_for_user(user_uuid)
        .await?
        .expect("should find latest token for user");
    assert_eq!(latest.id, id);

    // Mark used
    repo.mark_used(id).await?;
    let used = repo
        .find_by_token_hash(token_hash)
        .await?
        .expect("token should still exist after marking used");
    assert!(
        used.used_at.is_some(),
        "token should have used_at set after mark_used"
    );

    // Delete for user
    repo.delete_for_user(user_uuid).await?;
    assert!(
        repo.find_by_token_hash(token_hash).await?.is_none(),
        "token should be gone after delete_for_user"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_delete_expired_tokens() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = PasswordResetRepository::new(pool.pool.clone());

    // Insert an already-expired token
    let expires_at = time::OffsetDateTime::now_utc() - Duration::hours(1);
    repo.insert_token(user_uuid, "expired_hash_test", expires_at)
        .await?;

    // Insert a still-valid token
    let valid_expires_at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    repo.insert_token(user_uuid, "valid_hash_test", valid_expires_at)
        .await?;

    // Delete expired — only the past-expiry token should be removed
    let deleted = repo.delete_expired().await?;
    assert_eq!(deleted, 1, "exactly one expired token should be deleted");

    // Valid token should still exist
    assert!(
        repo.find_by_token_hash("valid_hash_test").await?.is_some(),
        "valid token should survive delete_expired"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_password_reset_find_by_hash_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = PasswordResetRepository::new(pool.pool.clone());

    let result = repo.find_by_token_hash("nonexistent_hash").await?;
    assert!(
        result.is_none(),
        "find_by_token_hash should return None for unknown hash"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_password_reset_find_latest_for_user_multiple_tokens() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = PasswordResetRepository::new(pool.pool.clone());

    let expires_at = time::OffsetDateTime::now_utc() + Duration::hours(1);

    // Insert two tokens — second should be latest
    repo.insert_token(user_uuid, "first_hash_multi", expires_at)
        .await?;

    // Small delay to ensure ordering is deterministic by created_at
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let second_id = repo
        .insert_token(user_uuid, "second_hash_multi", expires_at)
        .await?;

    let latest = repo
        .find_latest_for_user(user_uuid)
        .await?
        .expect("should find latest token");
    assert_eq!(
        latest.id, second_id,
        "find_latest_for_user should return the most recently inserted token"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_password_reset_delete_for_user_clears_all_tokens() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = PasswordResetRepository::new(pool.pool.clone());

    let expires_at = time::OffsetDateTime::now_utc() + Duration::hours(1);

    repo.insert_token(user_uuid, "token_del_1", expires_at)
        .await?;
    repo.insert_token(user_uuid, "token_del_2", expires_at)
        .await?;

    repo.delete_for_user(user_uuid).await?;

    assert!(
        repo.find_by_token_hash("token_del_1").await?.is_none(),
        "first token should be deleted"
    );
    assert!(
        repo.find_by_token_hash("token_del_2").await?.is_none(),
        "second token should be deleted"
    );
    assert!(
        repo.find_latest_for_user(user_uuid).await?.is_none(),
        "no tokens should remain for user"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_password_reset_token_has_expiry_fields() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = PasswordResetRepository::new(pool.pool.clone());

    let expires_at = time::OffsetDateTime::now_utc() + Duration::hours(2);
    let _id = repo
        .insert_token(user_uuid, "expiry_field_hash", expires_at)
        .await?;

    let token = repo
        .find_by_token_hash("expiry_field_hash")
        .await?
        .expect("token should exist");

    assert!(
        token.expires_at > time::OffsetDateTime::now_utc(),
        "expires_at should be in the future"
    );
    assert!(
        token.created_at <= time::OffsetDateTime::now_utc(),
        "created_at should be in the past or present"
    );
    assert!(token.used_at.is_none(), "used_at should be None initially");

    Ok(())
}

/// A locked account that resets its password is unlocked by the reset flow
/// (`PasswordResetService::reset_password` clears the lockout state).
#[tokio::test]
#[serial]
async fn test_reset_password_unlocks_locked_account() -> Result<()> {
    use r_data_core_core::admin_user::UserStatus;
    use r_data_core_core::config::parse_smtp_dsn;
    use r_data_core_persistence::{
        AdminUserRepository, AdminUserRepositoryTrait, EmailTemplateRepository,
    };
    use r_data_core_services::{MailService, PasswordResetService};
    use r_data_core_test_support::test_queue_client_async;
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    use time::OffsetDateTime;

    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool.pool).await?;
    let user_repo = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    // Lock the account.
    user_repo
        .update_lockout_state(&user_uuid, &UserStatus::Locked, 5)
        .await?;

    // Build the service. The reset path only touches the repos; mail/template/
    // queue are constructed but unused (no SMTP I/O — tls=false builds a transport).
    let reset_repo = Arc::new(PasswordResetRepository::new(pool.pool.clone()));
    let smtp =
        parse_smtp_dsn("smtp://localhost:1025?tls=false&from=test@example.com&from_name=Test")
            .expect("smtp dsn");
    let service = PasswordResetService::new(
        reset_repo.clone(),
        user_repo.clone(),
        Arc::new(EmailTemplateRepository::new(pool.pool.clone())),
        test_queue_client_async().await,
        Arc::new(MailService::new(&smtp).expect("mail service")),
        60,
        "http://localhost".to_string(),
    );

    // Seed a valid token whose SHA-256 matches the raw token the caller submits.
    let raw_token = "reset-token-unlock-abc123";
    let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    reset_repo
        .insert_token(
            user_uuid,
            &token_hash,
            OffsetDateTime::now_utc() + Duration::hours(1),
        )
        .await?;

    service
        .reset_password(raw_token, "new-strong-password-123")
        .await?;

    let user = user_repo
        .find_by_uuid(&user_uuid)
        .await?
        .expect("user exists");
    assert_eq!(
        user.status,
        UserStatus::Active,
        "password reset must unlock a locked account"
    );
    assert_eq!(
        user.failed_login_attempts, 0,
        "password reset must clear the failed-attempt counter"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
