#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

use r_data_core_core::admin_user::UserStatus;
use r_data_core_core::error::Result;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

pub fn make_repo(pool: &r_data_core_test_support::TestDatabase) -> AdminUserRepository {
    AdminUserRepository::new(Arc::new(pool.pool.clone()))
}

pub async fn seed_user(
    repo: &AdminUserRepository,
    pool: &r_data_core_test_support::TestDatabase,
) -> Result<Uuid> {
    let creator = create_test_admin_user(&pool.pool).await?;
    let username = random_string("usr");
    let email = format!("{}@test.example", random_string("mail"));
    let params = CreateAdminUserParams {
        username: &username,
        email: &email,
        password: "correct-horse-battery",
        first_name: "Ada",
        last_name: "Lovelace",
        role: None,
        is_active: true,
        creator_uuid: creator,
    };
    repo.create_admin_user(&params).await
}

// ── find_by_username_or_email ──────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_find_by_email() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let creator = create_test_admin_user(&pool.pool).await?;
    let username = random_string("u");
    let email = format!("{}@find.test", random_string("m"));
    let uuid = repo
        .create_admin_user(&CreateAdminUserParams {
            username: &username,
            email: &email,
            password: "passw0rd-long",
            first_name: "A",
            last_name: "B",
            role: None,
            is_active: true,
            creator_uuid: creator,
        })
        .await?;

    let found = repo.find_by_username_or_email(&email).await?;
    assert!(found.is_some(), "should find user by email");
    assert_eq!(found.unwrap().uuid, uuid);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_find_by_username_or_email_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let result = repo
        .find_by_username_or_email("nobody@nowhere.invalid")
        .await?;
    assert!(result.is_none(), "non-existent user should return None");
    Ok(())
}

// ── find_by_uuid ───────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_find_by_uuid_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let result = repo.find_by_uuid(&Uuid::now_v7()).await?;
    assert!(result.is_none(), "random UUID should not match any user");
    Ok(())
}

// ── create_admin_user: self-creation (nil creator) ─────────────────────────

#[tokio::test]
#[serial]
async fn test_create_user_self_creation() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let username = random_string("self");
    let email = format!("{}@self.test", random_string("s"));
    let uuid = repo
        .create_admin_user(&CreateAdminUserParams {
            username: &username,
            email: &email,
            password: "correct-horse-battery",
            first_name: "Self",
            last_name: "Made",
            role: None,
            is_active: true,
            creator_uuid: Uuid::nil(), // triggers self-reference path
        })
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.uuid, uuid, "UUID round-trips");
    Ok(())
}

// ── create_admin_user: inactive user ──────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_create_inactive_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let creator = create_test_admin_user(&pool.pool).await?;
    let username = random_string("inactive");
    let email = format!("{}@inactive.test", random_string("i"));
    let uuid = repo
        .create_admin_user(&CreateAdminUserParams {
            username: &username,
            email: &email,
            password: "correct-horse-battery",
            first_name: "In",
            last_name: "Active",
            role: None,
            is_active: false,
            creator_uuid: creator,
        })
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert!(!user.is_active, "user should be inactive");
    Ok(())
}

// ── update_admin_user ──────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_update_admin_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    let mut user = repo.find_by_uuid(&uuid).await?.expect("user must exist");

    user.username = random_string("updated_user");
    user.email = format!("{}@updated.test", random_string("upd"));
    user.first_name = Some("Updated".to_string());
    user.last_name = Some("Name".to_string());
    user.super_admin = true;

    repo.update_admin_user(&user).await?;

    let refreshed = repo
        .find_by_uuid(&uuid)
        .await?
        .expect("user must still exist");
    assert_eq!(refreshed.username, user.username);
    assert_eq!(refreshed.email, user.email);
    assert_eq!(refreshed.first_name.as_deref(), Some("Updated"));
    assert!(refreshed.super_admin, "super_admin flag should be updated");
    Ok(())
}

// ── update_lockout_state ───────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_update_lockout_state_locked() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5)
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Locked);
    assert_eq!(user.failed_login_attempts, 5);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_lockout_state_active() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5)
        .await?;
    repo.update_lockout_state(&uuid, &UserStatus::Active, 0)
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Active);
    assert_eq!(user.failed_login_attempts, 0);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_lockout_state_pending_activation() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    repo.update_lockout_state(&uuid, &UserStatus::PendingActivation, 0)
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::PendingActivation);
    Ok(())
}
