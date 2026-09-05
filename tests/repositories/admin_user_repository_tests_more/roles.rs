#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, random_string, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

use super::users::{make_repo, seed_user};

// ── role management ────────────────────────────────────────────────────────

async fn seed_role(
    pool: &r_data_core_test_support::TestDatabase,
    created_by: Uuid,
) -> Result<Uuid> {
    let role_uuid = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles (uuid, name, description, super_admin, created_by, created_at, updated_at, published)
         VALUES ($1, $2, $3, false, $4, NOW(), NOW(), true)",
    )
    .bind(role_uuid)
    .bind(random_string("role"))
    .bind("test role")
    .bind(created_by)
    .execute(&pool.pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;
    Ok(role_uuid)
}

#[tokio::test]
#[serial]
async fn test_assign_and_get_user_roles() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_uuid).await?;

    let roles = repo.get_user_roles(user_uuid).await?;
    assert!(roles.contains(&role_uuid), "role should be assigned");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_assign_role_idempotent() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_uuid).await?;
    repo.assign_role(user_uuid, role_uuid).await?; // ON CONFLICT DO NOTHING

    let roles = repo.get_user_roles(user_uuid).await?;
    assert_eq!(
        roles.iter().filter(|&&r| r == role_uuid).count(),
        1,
        "duplicate assign must not create two rows"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_unassign_role() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_uuid).await?;
    repo.unassign_role(user_uuid, role_uuid).await?;

    let roles = repo.get_user_roles(user_uuid).await?;
    assert!(!roles.contains(&role_uuid), "role should be removed");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_users_by_role() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_uuid).await?;

    let users = repo.get_users_by_role(role_uuid).await?;
    assert!(
        users.contains(&user_uuid),
        "user should appear in role members"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_user_roles_replaces_existing() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_a = seed_role(&pool, user_uuid).await?;
    let role_b = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_a).await?;
    repo.update_user_roles(user_uuid, &[role_b]).await?;

    let roles = repo.get_user_roles(user_uuid).await?;
    assert!(!roles.contains(&role_a), "old role should be removed");
    assert!(roles.contains(&role_b), "new role should be present");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_user_roles_empty_clears_all() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    repo.assign_role(user_uuid, role_uuid).await?;
    repo.update_user_roles(user_uuid, &[]).await?;

    let roles = repo.get_user_roles(user_uuid).await?;
    assert!(roles.is_empty(), "all roles should be cleared");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_user_roles_empty_for_new_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let roles = repo.get_user_roles(user_uuid).await?;
    assert!(roles.is_empty(), "new user should have no roles initially");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_users_by_role_empty_for_unused_role() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let user_uuid = seed_user(&repo, &pool).await?;
    let role_uuid = seed_role(&pool, user_uuid).await?;

    // Role exists but has no users assigned
    let users = repo.get_users_by_role(role_uuid).await?;
    assert!(users.is_empty(), "unused role should have no members");
    Ok(())
}
