#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_services::RoleService;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use std::sync::Arc;

/// Test edge cases: user with no roles, empty permissions, role with no permissions
#[tokio::test]
#[serial]
async fn test_empty_permissions_edge_cases() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let admin_user_uuid = create_test_admin_user(&pool).await?;
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let role_service = RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600));

    // Create a user with no roles
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "noroleuser",
            email: "norole@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // User with no roles should return empty
    let roles = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles.len(), 0, "User with no roles should return empty");

    // Merged permissions for user with no roles should be empty
    let perms = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(
        perms.len(),
        0,
        "User with no roles should have no permissions"
    );

    // Create a role with no permissions
    let mut empty_role = Role::new("EmptyRole".to_string());
    empty_role.permissions = vec![]; // Empty permissions

    let empty_role_uuid = role_service
        .create_role(&empty_role, admin_user_uuid)
        .await?;

    // Assign empty role to user
    user_repo.assign_role(user_uuid, empty_role_uuid).await?;

    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    // User with role but no permissions should have empty permissions
    let perms2 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(
        perms2.len(),
        0,
        "User with role but no permissions should have empty permissions"
    );

    // User with role but no permissions should have empty permissions
    let perms3 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(
        perms3.len(),
        0,
        "User with role but no permissions should have empty permissions"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
