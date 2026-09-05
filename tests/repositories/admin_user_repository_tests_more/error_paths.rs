#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Every repository method against an unreachable database.
//!
//! The persistence crate denies `unwrap`/`expect`/`panic`, so a database that
//! is simply not there must surface as an `Err` from every entry point rather
//! than taking the process down. These are the `map_err` arms that the
//! happy-path tests never reach.

use r_data_core_core::admin_user::UserStatus;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

/// A pool pointed at a port nothing listens on. `connect_lazy` defers the
/// failure to first use, so construction succeeds and every query fails.
fn unreachable_repo() -> AdminUserRepository {
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(300))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/nonexistent")
        .expect("a lazy pool is built without contacting the server");

    AdminUserRepository::new(Arc::new(pool))
}

#[tokio::test]
async fn test_lookups_error_rather_than_panic() {
    let repo = unreachable_repo();
    let uuid = Uuid::now_v7();

    assert!(repo.find_by_username_or_email("someone").await.is_err());
    assert!(repo.find_by_uuid(&uuid).await.is_err());
    assert!(repo.list_admin_users(10, 0, None, None).await.is_err());
    assert!(repo.get_user_roles(uuid).await.is_err());
    assert!(repo.get_users_by_role(uuid).await.is_err());
}

#[tokio::test]
async fn test_writes_error_rather_than_panic() {
    let repo = unreachable_repo();
    let uuid = Uuid::now_v7();

    assert!(repo.update_last_login(&uuid).await.is_err());
    assert!(repo
        .update_lockout_state(&uuid, &UserStatus::Locked, 3, None)
        .await
        .is_err());
    assert!(repo.set_active(&uuid, false).await.is_err());
    assert!(repo.reset_password(&uuid, "hash").await.is_err());
    assert!(repo.delete_admin_user(&uuid).await.is_err());
}

#[tokio::test]
async fn test_role_assignment_errors_rather_than_panics() {
    let repo = unreachable_repo();
    let user = Uuid::now_v7();
    let role = Uuid::now_v7();

    assert!(repo.assign_role(user, role).await.is_err());
    assert!(repo.unassign_role(user, role).await.is_err());
    assert!(repo.update_user_roles(user, &[role]).await.is_err());
}

/// `update_user_roles` opens a transaction, so its failure path differs from
/// the single-statement writes above.
#[tokio::test]
async fn test_transactional_update_errors_rather_than_panics() {
    let repo = unreachable_repo();

    assert!(repo.update_user_roles(Uuid::now_v7(), &[]).await.is_err());
}

#[tokio::test]
async fn test_create_errors_rather_than_panics() {
    let repo = unreachable_repo();

    let params = CreateAdminUserParams {
        username: "someone",
        email: "someone@example.com",
        password: "a_long_enough_password",
        first_name: "Some",
        last_name: "One",
        role: None,
        is_active: true,
        creator_uuid: Uuid::now_v7(),
    };

    assert!(repo.create_admin_user(&params).await.is_err());
}
