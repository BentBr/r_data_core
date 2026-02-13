#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{http::StatusCode, test};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_services::RoleService;
use r_data_core_test_support::clear_test_db;
use serial_test::serial;
use std::sync::Arc;

use super::common::{create_test_role, setup_test_app};
use r_data_core_core::admin_jwt::generate_access_token;

/// Test successful authentication with roles
#[tokio::test]
#[serial]
async fn test_successful_auth_with_roles() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "testuser",
            email: "test@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"), // Set role to Editor
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role
    let role_service = RoleService::new(
        pool.pool.clone(),
        Arc::new(CacheManager::new(CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        })),
        Some(3600),
    );

    let mut role = create_test_role("TestRole");
    // Add permission to read roles
    role.permissions.push(Permission {
        resource_type: ResourceNamespace::Roles,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service.create_role(&role, admin_user_uuid).await?;
    role.base.uuid = role_uuid; // Set UUID for later updates

    // Assign role to user
    user_repo.assign_role(user_uuid, role_uuid).await?;

    // Generate JWT token with roles
    let roles = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    let api_config = r_data_core_core::config::ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: "test_secret".to_string(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
        check_default_admin_password: true,
    };
    let token = generate_access_token(&user, &api_config, &roles)?;

    // Test accessing a protected endpoint with valid permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/roles")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Should have permission to read roles"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test failing authentication (invalid token)
#[tokio::test]
#[serial]
async fn test_failing_auth_invalid_token() -> Result<()> {
    let (app, pool, _) = setup_test_app().await?;

    // Try to access protected endpoint with invalid token
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/roles")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Should reject invalid token"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test failing permissions (user without required permission)
#[tokio::test]
#[serial]
async fn test_failing_permissions_no_permission() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user without super_admin
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "testuser2",
            email: "test2@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"), // Set role
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin and no roles assigned
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Generate JWT token without any roles (no permissions)
    let api_config = r_data_core_core::config::ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: "test_secret".to_string(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
        check_default_admin_password: true,
    };
    let token = generate_access_token(&user, &api_config, &[])?;

    // Try to access protected endpoint without permission
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/roles")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Should reject request without required permission"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test successful auth with different roles
#[tokio::test]
#[serial]
async fn test_successful_auth_with_different_roles() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "testuser3",
            email: "test3@example.com",
            password: "password123",
            first_name: "Test",
            last_name: "User",
            role: Some("Editor"), // Set role to Editor
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    let role_service = RoleService::new(
        pool.pool.clone(),
        Arc::new(CacheManager::new(CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        })),
        Some(3600),
    );

    // Create two different roles
    let mut role1 = Role::new("Role1".to_string());
    role1.permissions = vec![Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];
    let role1_uuid = role_service.create_role(&role1, admin_user_uuid).await?;

    let mut role2 = Role::new("Role2".to_string());
    role2.permissions = vec![Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Create,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];
    let role2_uuid = role_service.create_role(&role2, admin_user_uuid).await?;

    // Assign both roles to user
    user_repo.assign_role(user_uuid, role1_uuid).await?;
    user_repo.assign_role(user_uuid, role2_uuid).await?;

    // Generate JWT with merged permissions from both roles
    let roles = role_service
        .get_roles_for_user(user_uuid, &user_repo)
        .await?;
    assert_eq!(roles.len(), 2, "User should have 2 roles");

    let api_config = r_data_core_core::config::ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: "test_secret".to_string(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
        check_default_admin_password: true,
    };
    let token = generate_access_token(&user, &api_config, &roles)?;

    // Test that user has both Read and Create permissions (merged from both roles)
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Should have Read permission from role1"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
