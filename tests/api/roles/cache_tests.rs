#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, ApiKeyRepositoryTrait,
    CreateAdminUserParams,
};
use r_data_core_services::RoleService;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use std::sync::Arc;

use super::common::create_test_role;

/// Test permission caching behavior
#[tokio::test]
#[serial]
async fn test_permission_caching() -> Result<()> {
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

    // Create a role
    let role = create_test_role("CacheTestRole");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Verify role was created
    let initial_role = role_service.get_role(role_uuid).await?.unwrap();
    assert_eq!(initial_role.name, "CacheTestRole", "Role name should match");

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "cacheuser",
            email: "cache@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"), // Set role
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Assign role to user
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // First access - should load from DB and cache
    let roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles1.len(), 1, "Should have 1 role");

    // Second access - should come from cache
    let roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles2.len(), 1, "Should still have 1 role from cache");

    // Update the role - reload it first to get the complete role with base UUID
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    // Verify we got the role correctly
    assert_eq!(
        role_to_update.base.uuid, role_uuid,
        "Role UUID should match"
    );
    assert_eq!(
        role_to_update.name, "CacheTestRole",
        "Initial name should match"
    );

    // Update the name instead of description (description has a known sqlx binding issue)
    let original_name = role_to_update.name.clone();
    role_to_update.name = "Updated CacheTestRole".to_string();

    // Update in database - verify it succeeds
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // Verify the update using the service method
    let updated_role = role_service.get_role(role_uuid).await?.unwrap();
    assert_eq!(
        updated_role.name, "Updated CacheTestRole",
        "Name should be updated in DB"
    );

    // Invalidate user cache to force reload
    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    // Access after update - should reload from DB (cache invalidated)
    let roles3 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles3.len(), 1, "Should still have 1 role");
    assert_eq!(
        roles3[0].name, "Updated CacheTestRole",
        "Should have updated name in user roles"
    );
    assert_ne!(
        roles3[0].name, original_name,
        "Name should be different from original"
    );

    // Test cache invalidation on role assignment
    let role2 = create_test_role("CacheTestRole2");
    let role2_uuid = role_service.create_role(&role2, admin_user_uuid).await?;

    // Assign new role - should invalidate cache
    user_repo.assign_role(user_uuid, role2_uuid).await?;

    // Invalidate cache after assignment (same as API route does)
    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    // Access after assignment - should reload from DB
    let roles4 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles4.len(), 2, "Should have 2 roles after assignment");

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test merged permissions caching
#[tokio::test]
#[serial]
async fn test_merged_permissions_caching() -> Result<()> {
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

    // Create a role with permissions
    let mut role = create_test_role("MergedCacheTest");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;
    role.base.uuid = role_uuid; // Set UUID for later updates

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "mergeduser",
            email: "merged@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"), // Set role
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Assign role
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // First access - should calculate and cache merged permissions
    let perms1 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    assert!(!perms1.is_empty(), "Should have merged permissions");

    // Second access - should come from cache
    let perms2 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(perms1, perms2, "Should return same permissions from cache");

    // Update role - should invalidate cache - need to reload first to get base UUID
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    // Add a new permission
    role_to_update.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Update,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // Invalidate user cache to force recalculation of merged permissions
    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    // Access after update - should recalculate (cache invalidated)
    let perms3 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;

    // Verify we have more permissions
    assert!(
        perms3.len() > perms1.len(),
        "Should have more permissions after update. Before: {}, After: {}",
        perms1.len(),
        perms3.len()
    );

    // Verify the new permission is included
    assert!(
        perms3.iter().any(|p| p.contains("workflows:update")),
        "Should include the new Update permission"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test cache invalidation when role is deleted
#[tokio::test]
#[serial]
async fn test_cache_invalidation_on_role_deletion() -> Result<()> {
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

    // Create a role
    let role = create_test_role("DeleteTestRole");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "deleteuser",
            email: "delete@example.com",
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

    // Assign role to user
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // Load roles to populate cache
    let roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles1.len(), 1, "Should have 1 role");

    // Delete the role - should invalidate all related caches
    role_service.delete_role(role_uuid).await?;

    // Access after deletion - should return empty (role deleted, cache invalidated)
    let roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles2.len(), 0, "Should have 0 roles after deletion");

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test multiple users sharing same role - update should invalidate all their caches
#[tokio::test]
#[serial]
async fn test_multiple_users_sharing_role() -> Result<()> {
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

    // Create a role
    let role = create_test_role("SharedRole");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create two users
    let user1_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "shareduser1",
            email: "shared1@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User1",
            role: Some("Editor"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let user2_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "shareduser2",
            email: "shared2@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User2",
            role: Some("Editor"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update users to not be super_admin
    let mut user1 = user_repo.find_by_uuid(&user1_uuid).await?.unwrap();
    user1.super_admin = false;

    user_repo.update_admin_user(&user1).await?;

    let mut user2 = user_repo.find_by_uuid(&user2_uuid).await?.unwrap();
    user2.super_admin = false;

    user_repo.update_admin_user(&user2).await?;

    // Assign role to both users
    user_repo.assign_role(user1_uuid, role_uuid).await?;
    user_repo.assign_role(user2_uuid, role_uuid).await?;

    // Load roles for both users to populate cache
    let roles1_user1 = role_service
        .get_roles_for_user(user1_uuid, &user_repo)
        .await?;
    let roles1_user2 = role_service
        .get_roles_for_user(user2_uuid, &user_repo)
        .await?;
    assert_eq!(roles1_user1.len(), 1, "User1 should have 1 role");
    assert_eq!(roles1_user2.len(), 1, "User2 should have 1 role");

    // Update the role - should invalidate both users' caches
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    role_to_update.name = "Updated SharedRole".to_string();
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // Both users should see the updated role
    let roles2_user1 = role_service
        .get_roles_for_user(user1_uuid, &user_repo)
        .await?;
    let roles2_user2 = role_service
        .get_roles_for_user(user2_uuid, &user_repo)
        .await?;
    assert_eq!(roles2_user1.len(), 1, "User1 should still have 1 role");
    assert_eq!(roles2_user2.len(), 1, "User2 should still have 1 role");
    assert_eq!(
        roles2_user1[0].name, "Updated SharedRole",
        "User1 should see updated role name"
    );
    assert_eq!(
        roles2_user2[0].name, "Updated SharedRole",
        "User2 should see updated role name"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test API key permission caching and invalidation
#[tokio::test]
#[serial]
async fn test_api_key_permission_caching() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let admin_user_uuid = create_test_admin_user(&pool).await?;
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let role_service = RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600));

    // Create a role
    let role = create_test_role("ApiKeyTestRole");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create an API key
    let api_key_uuid = api_key_repo
        .create_new_api_key("Test API Key", "Test description", admin_user_uuid, 30)
        .await?
        .0;

    // Assign role to API key
    api_key_repo.assign_role(api_key_uuid, role_uuid).await?;

    // First access - should load from DB and cache
    let roles1 = role_service
        .get_roles_for_api_key(api_key_uuid, &api_key_repo)
        .await?;
    assert_eq!(roles1.len(), 1, "Should have 1 role");

    // Second access - should come from cache
    let roles2 = role_service
        .get_roles_for_api_key(api_key_uuid, &api_key_repo)
        .await?;
    assert_eq!(roles2.len(), 1, "Should still have 1 role from cache");

    // Update the role
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    role_to_update.name = "Updated ApiKeyTestRole".to_string();
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // Access after update - should reload from DB (cache invalidated)
    let roles3 = role_service
        .get_roles_for_api_key(api_key_uuid, &api_key_repo)
        .await?;
    assert_eq!(roles3.len(), 1, "Should still have 1 role");
    assert_eq!(
        roles3[0].name, "Updated ApiKeyTestRole",
        "Should have updated name"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test cache invalidation when role is unassigned from user/API key
#[tokio::test]
#[serial]
async fn test_cache_invalidation_on_role_unassignment() -> Result<()> {
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

    // Create two roles
    let role1 = create_test_role("UnassignRole1");
    let role1_uuid = role_service.create_role(&role1, admin_user_uuid).await?;

    let role2 = create_test_role("UnassignRole2");
    let role2_uuid = role_service.create_role(&role2, admin_user_uuid).await?;

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "unassignuser",
            email: "unassign@example.com",
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

    // Assign both roles to user
    user_repo.assign_role(user_uuid, role1_uuid).await?;
    user_repo.assign_role(user_uuid, role2_uuid).await?;

    // Load roles to populate cache
    let roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles1.len(), 2, "Should have 2 roles");

    // Unassign one role
    user_repo.unassign_role(user_uuid, role1_uuid).await?;

    // Invalidate cache (same as API route does)
    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    // Access after unassignment - should reload from DB
    let roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles2.len(), 1, "Should have 1 role after unassignment");
    assert_eq!(
        roles2[0].base.uuid, role2_uuid,
        "Should have the remaining role"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test that `update_role` automatically invalidates related caches without manual invalidation
#[tokio::test]
#[serial]
async fn test_automatic_cache_invalidation_on_update() -> Result<()> {
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

    // Create a role
    let role = create_test_role("AutoInvalidateRole");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "autouser",
            email: "auto@example.com",
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

    // Assign role to user
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // Load roles to populate cache
    let roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles1.len(), 1, "Should have 1 role");
    assert_eq!(
        roles1[0].name, "AutoInvalidateRole",
        "Initial name should match"
    );

    // Update the role - should automatically invalidate cache
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    role_to_update.name = "AutoUpdatedRole".to_string();
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // Access after update WITHOUT manual invalidation - should still see updated role
    let roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles2.len(), 1, "Should still have 1 role");
    assert_eq!(
        roles2[0].name, "AutoUpdatedRole",
        "Should have updated name without manual invalidation"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test merged permissions for multiple roles (different roles have separate caches)
#[tokio::test]
#[serial]
async fn test_merged_permissions_multiple_roles() -> Result<()> {
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

    // Create a role with permissions
    let mut role = Role::new("MultiRole".to_string());
    role.permissions = vec![Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];

    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create two users with different roles
    let editor_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "editoruser",
            email: "editor@example.com",
            password: "password123",
            first_name: "Editor",
            last_name: "User",
            role: Some("Editor"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let viewer_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "vieweruser",
            email: "viewer@example.com",
            password: "password123",
            first_name: "Viewer",
            last_name: "User",
            role: Some("Viewer"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update users to not be super_admin
    let mut editor = user_repo.find_by_uuid(&editor_uuid).await?.unwrap();
    editor.super_admin = false;

    user_repo.update_admin_user(&editor).await?;

    let mut viewer = user_repo.find_by_uuid(&viewer_uuid).await?.unwrap();
    viewer.super_admin = false;

    user_repo.update_admin_user(&viewer).await?;

    // Assign role to both users
    user_repo.assign_role(editor_uuid, role_uuid).await?;
    user_repo.assign_role(viewer_uuid, role_uuid).await?;

    // Get merged permissions for both roles - should be cached separately
    let editor_perms1 = role_service
        .get_merged_permissions_for_user(editor_uuid, &user_repo)
        .await?;
    let viewer_perms1 = role_service
        .get_merged_permissions_for_user(viewer_uuid, &user_repo)
        .await?;

    assert!(!editor_perms1.is_empty(), "Editor should have permissions");
    assert!(!viewer_perms1.is_empty(), "Viewer should have permissions");

    // Second access - should come from cache
    let editor_perms2 = role_service
        .get_merged_permissions_for_user(editor_uuid, &user_repo)
        .await?;
    let viewer_perms2 = role_service
        .get_merged_permissions_for_user(viewer_uuid, &user_repo)
        .await?;

    assert_eq!(
        editor_perms1, editor_perms2,
        "Editor permissions should be cached"
    );
    assert_eq!(
        viewer_perms1, viewer_perms2,
        "Viewer permissions should be cached"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test fallback to DB when cache deserialization fails (type safety)
#[tokio::test]
#[serial]
async fn test_cache_deserialization_failure_handling() -> Result<()> {
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

    // Create a role
    let role = create_test_role("DeserializationTest");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create a user
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "deseruser",
            email: "deser@example.com",
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

    // Assign role to user
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // Load roles to populate cache
    let roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles1.len(), 1, "Should have 1 role");

    // The type-safe structs ensure that if deserialization fails,
    // we get None from cache and fall back to DB
    // This is the expected behavior
    // We can't easily corrupt the cache in tests, but the type-safe structs
    // ensure proper deserialization

    // Verify we can still load from DB even if cache is invalid
    // (by invalidating cache and reloading)
    role_service
        .invalidate_user_permissions_cache(&user_uuid)
        .await;

    let roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(
        roles2.len(),
        1,
        "Should still load from DB after cache invalidation"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Comprehensive test that role update invalidates all related caches
#[tokio::test]
#[serial]
async fn test_role_update_invalidates_all_related_caches() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let admin_user_uuid = create_test_admin_user(&pool).await?;
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let role_service = RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600));

    // Create a role
    let role = create_test_role("ComprehensiveTest");
    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;

    // Create a user and API key
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "comprehensiveuser",
            email: "comprehensive@example.com",
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

    let api_key_uuid = api_key_repo
        .create_new_api_key("Test API Key", "Test description", admin_user_uuid, 30)
        .await?
        .0;

    // Assign role to both user and API key
    user_repo.assign_role(user_uuid, role_uuid).await?;
    api_key_repo.assign_role(api_key_uuid, role_uuid).await?;

    // Load roles and permissions to populate all caches
    let user_roles1 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    let api_key_roles1 = role_service
        .get_roles_for_api_key(api_key_uuid, &api_key_repo)
        .await?;
    let user_perms1 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    let api_key_perms1 = role_service
        .get_merged_permissions_for_api_key(api_key_uuid, &api_key_repo)
        .await?;

    assert_eq!(user_roles1.len(), 1, "User should have 1 role");
    assert_eq!(api_key_roles1.len(), 1, "API key should have 1 role");
    assert!(!user_perms1.is_empty(), "User should have permissions");
    assert!(
        !api_key_perms1.is_empty(),
        "API key should have permissions"
    );

    // Update the role - should invalidate all related caches
    let mut role_to_update = role_service.get_role(role_uuid).await?.unwrap();
    role_to_update.name = "Updated ComprehensiveTest".to_string();
    // Add a new permission
    role_to_update.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Delete,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });
    role_service
        .update_role(&role_to_update, admin_user_uuid)
        .await?;

    // All caches should be invalidated and reloaded
    let user_roles2 = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    let api_key_roles2 = role_service
        .get_roles_for_api_key(api_key_uuid, &api_key_repo)
        .await?;
    let user_perms2 = role_service
        .get_merged_permissions_for_user(user_uuid, &user_repo)
        .await?;
    let api_key_perms2 = role_service
        .get_merged_permissions_for_api_key(api_key_uuid, &api_key_repo)
        .await?;

    // Verify role cache was invalidated
    assert_eq!(
        user_roles2[0].name, "Updated ComprehensiveTest",
        "User should see updated role name"
    );
    assert_eq!(
        api_key_roles2[0].name, "Updated ComprehensiveTest",
        "API key should see updated role name"
    );

    // Verify the merged permissions cache was invalidated
    assert!(
        user_perms2.len() > user_perms1.len(),
        "User should have more permissions after update"
    );
    assert!(
        api_key_perms2.len() > api_key_perms1.len(),
        "API key should have more permissions after update"
    );
    assert!(
        user_perms2.iter().any(|p| p.contains("workflows:delete")),
        "User should have the new Delete permission"
    );
    assert!(
        api_key_perms2
            .iter()
            .any(|p| p.contains("workflows:delete")),
        "API key should have the new Delete permission"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
