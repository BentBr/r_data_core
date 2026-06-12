#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_role(pool: &PgPool, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO roles (name, created_by, published) VALUES ($1, $2, true) RETURNING uuid",
    )
    .bind(random_string("role"))
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("failed to create test role")
}

#[tokio::test]
#[serial]
async fn test_roles_assign_get_unassign() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("role_key"), "desc", user_uuid, 30)
        .await?;

    let roles_before = repo.get_api_key_roles(key_uuid).await?;
    assert!(roles_before.is_empty());

    let role1 = create_test_role(&pool.pool, user_uuid).await;
    let role2 = create_test_role(&pool.pool, user_uuid).await;
    repo.assign_role(key_uuid, role1).await?;
    repo.assign_role(key_uuid, role2).await?;

    let roles_after = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(roles_after.len(), 2);
    assert!(roles_after.contains(&role1));
    assert!(roles_after.contains(&role2));

    repo.unassign_role(key_uuid, role1).await?;
    let roles_final = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(roles_final.len(), 1);
    assert!(roles_final.contains(&role2));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_assign_role_idempotent() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;
    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("idem_key"), "desc", user_uuid, 30)
        .await?;

    let role = create_test_role(&pool.pool, user_uuid).await;

    // Assign twice — should not fail (ON CONFLICT DO NOTHING)
    repo.assign_role(key_uuid, role).await?;
    repo.assign_role(key_uuid, role).await?;

    let roles = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(roles.len(), 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_api_key_roles_replaces_all() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;
    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("replace_key"), "desc", user_uuid, 30)
        .await?;

    let role1 = create_test_role(&pool.pool, user_uuid).await;
    let role2 = create_test_role(&pool.pool, user_uuid).await;
    let role3 = create_test_role(&pool.pool, user_uuid).await;

    repo.assign_role(key_uuid, role1).await?;
    repo.assign_role(key_uuid, role2).await?;

    repo.update_api_key_roles(key_uuid, &[role3]).await?;

    let roles = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(roles.len(), 1);
    assert!(roles.contains(&role3));
    assert!(!roles.contains(&role1));
    assert!(!roles.contains(&role2));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_api_keys_by_role() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key1_uuid, _) = repo
        .create_new_api_key(&random_string("by_role_k1"), "d", user_uuid, 30)
        .await?;
    let (key2_uuid, _) = repo
        .create_new_api_key(&random_string("by_role_k2"), "d", user_uuid, 30)
        .await?;

    let role = create_test_role(&pool.pool, user_uuid).await;
    repo.assign_role(key1_uuid, role).await?;
    repo.assign_role(key2_uuid, role).await?;

    let keys_with_role = repo.get_api_keys_by_role(role).await?;
    assert_eq!(keys_with_role.len(), 2);
    assert!(keys_with_role.contains(&key1_uuid));
    assert!(keys_with_role.contains(&key2_uuid));

    // Unknown role UUID not in roles table → no assignments → empty vec
    let unknown_role = Uuid::now_v7();
    let empty = repo.get_api_keys_by_role(unknown_role).await?;
    assert!(empty.is_empty());

    Ok(())
}
