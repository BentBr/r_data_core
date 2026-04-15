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

use super::common::setup_test_app;
use r_data_core_core::admin_jwt::generate_access_token;

/// Integration test for Admin permission granting all permissions for a namespace
#[tokio::test]
#[serial]
async fn test_admin_permission_grants_all_permissions() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "admin_test_user",
            email: "admin_test@example.com",
            password: "password123",
            first_name: "Admin",
            last_name: "Test",
            role: Some("Admin"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role with Admin permission for Workflows namespace
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

    let mut admin_role = Role::new("WorkflowsAdmin".to_string());
    // Add Admin permission for Workflows namespace
    admin_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&admin_role, admin_user_uuid)
        .await?;

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

    // Test that Admin permission grants Read access to Workflows
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Admin permission should grant Read access to Workflows"
    );

    // Test that Admin permission grants access to workflows endpoint (which requires Read)
    // This tests that Admin is treated as including Read for route access
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Should be able to access permissions endpoint"
    );

    // Verify the permissions response includes workflows:admin and allowed_routes includes /workflows
    let body: serde_json::Value = test::read_body_json(resp).await;

    // The response structure is: { "data": { "is_super_admin": ..., "permissions": ..., "allowed_routes": ... } }
    let data = body.get("data").and_then(|d| d.as_object());
    let permissions: Vec<String> = data
        .and_then(|d| d.get("permissions"))
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    assert!(
        permissions.contains(&"workflows:admin".to_string()),
        "Permissions should include workflows:admin. Got: {permissions:?}"
    );

    let allowed_routes: Vec<String> = data
        .and_then(|d| d.get("allowed_routes"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    assert!(
        allowed_routes.contains(&"/workflows".to_string()),
        "Allowed routes should include /workflows (Admin grants Read for route access). Got: {allowed_routes:?}"
    );

    // Test that Admin permission does NOT grant access to other namespaces
    // Try to access entity-definitions endpoint (requires EntityDefinitions:Read, not Workflows:Admin)
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Admin permission for Workflows should NOT grant access to EntityDefinitions namespace"
    );

    // Test that Admin permission does NOT grant access to System namespace
    // Use dashboard stats endpoint which requires DashboardStats:Read (part of System namespace)
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Admin permission for Workflows should NOT grant access to System/DashboardStats namespace"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Integration test for Admin permission vs `super_admin` distinction
#[tokio::test]
#[serial]
async fn test_admin_vs_super_admin_distinction() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

    // Create a user with Admin permission for Workflows only (not super_admin)
    let user1_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "resource_admin",
            email: "resource_admin@example.com",
            password: "password123",
            first_name: "Resource",
            last_name: "Admin",
            role: Some("ResourceAdmin"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user1 = user_repo.find_by_uuid(&user1_uuid).await?.unwrap();
    user1.super_admin = false;
    user_repo.update_admin_user(&user1).await?;

    // Create a role with Admin permission for Workflows
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

    let mut workflows_admin_role = Role::new("WorkflowsAdminRole".to_string());
    workflows_admin_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&workflows_admin_role, admin_user_uuid)
        .await?;
    user_repo.assign_role(user1_uuid, role_uuid).await?;

    let roles1 = role_service
        .get_roles_for_user(user1_uuid, &user_repo)
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
    let token1 = generate_access_token(&user1, &api_config, &roles1)?;

    // User1 should have access to Workflows (has Admin permission)
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token1}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Resource-level Admin should grant access to Workflows"
    );

    // User1 should NOT have access to System (no Admin permission for System)
    // Use dashboard stats endpoint which requires DashboardStats:Read
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token1}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Resource-level Admin should NOT grant access to other namespaces"
    );

    // Create a super_admin user
    let user2_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "super_admin_user",
            email: "super_admin@example.com",
            password: "password123",
            first_name: "Super",
            last_name: "Admin",
            role: Some("SuperAdmin"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut user2 = user_repo.find_by_uuid(&user2_uuid).await?.unwrap();
    user2.super_admin = true; // Set as super_admin
    user_repo.update_admin_user(&user2).await?;

    let token2 = generate_access_token(&user2, &api_config, &[])?; // No roles needed for super_admin

    // User2 (super_admin) should have access to ALL namespaces
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token2}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Super admin should have access to Workflows"
    );

    // User2 (super_admin) should have access to System namespace
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token2}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Super admin should have access to System/DashboardStats (all namespaces)"
    );

    // User2 (super_admin) should have access to EntityDefinitions namespace
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header(("Authorization", format!("Bearer {token2}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Super admin should have access to EntityDefinitions (all namespaces)"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
