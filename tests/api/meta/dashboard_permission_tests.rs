#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

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

use crate::api::roles::common::setup_test_app;
use r_data_core_api::jwt::generate_access_token;

/// Test that users without `dashboard_stats:read` permission get 403 Forbidden
#[tokio::test]
#[serial]
async fn test_dashboard_without_permission_returns_forbidden() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "no_dashboard_user",
            email: "no_dashboard@example.com",
            password: "password123",
            first_name: "No",
            last_name: "Dashboard",
            role: Some("NoDashboard"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role with Workflows permission (but not DashboardStats)
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

    let mut workflows_role = Role::new("WorkflowsOnly".to_string());
    workflows_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&workflows_role, admin_user_uuid)
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

    // Test that user cannot access dashboard without permission
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User without dashboard_stats:read permission should get 403 Forbidden"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that users with `dashboard_stats:read` permission can access dashboard
#[tokio::test]
#[serial]
async fn test_dashboard_with_read_permission_returns_ok() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "dashboard_user",
            email: "dashboard@example.com",
            password: "password123",
            first_name: "Dashboard",
            last_name: "User",
            role: Some("DashboardUser"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role with DashboardStats:Read permission
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

    let mut dashboard_role = Role::new("DashboardReader".to_string());
    dashboard_role.permissions.push(Permission {
        resource_type: ResourceNamespace::DashboardStats,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&dashboard_role, admin_user_uuid)
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

    // Test that user can access dashboard with permission
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with dashboard_stats:read permission should be able to access dashboard"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that super admin users can access dashboard
#[tokio::test]
#[serial]
async fn test_dashboard_super_admin_returns_ok() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a super_admin user
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
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

    // Update user to be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = true;
    user_repo.update_admin_user(&user).await?;

    // Generate JWT token without roles (super_admin doesn't need roles)
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

    // Test that super admin can access dashboard
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Super admin should be able to access dashboard"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that users with `dashboard_stats:admin` permission can access dashboard
#[tokio::test]
#[serial]
async fn test_dashboard_with_admin_permission_returns_ok() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "dashboard_admin",
            email: "dashboard_admin@example.com",
            password: "password123",
            first_name: "Dashboard",
            last_name: "Admin",
            role: Some("DashboardAdmin"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role with DashboardStats:Admin permission
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

    let mut dashboard_admin_role = Role::new("DashboardAdminRole".to_string());
    dashboard_admin_role.permissions.push(Permission {
        resource_type: ResourceNamespace::DashboardStats,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&dashboard_admin_role, admin_user_uuid)
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

    // Test that user with Admin permission can access dashboard
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with dashboard_stats:admin permission should be able to access dashboard"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that users with other namespace permissions (e.g., workflows:read) cannot access dashboard
#[tokio::test]
#[serial]
async fn test_dashboard_other_namespace_permission_returns_forbidden() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a test user (not super_admin)
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "workflows_user",
            email: "workflows@example.com",
            password: "password123",
            first_name: "Workflows",
            last_name: "User",
            role: Some("WorkflowsUser"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
    let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
    user.super_admin = false;
    user_repo.update_admin_user(&user).await?;

    // Create a role with Workflows:Read permission (but not DashboardStats)
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

    let mut workflows_role = Role::new("WorkflowsReader".to_string());
    workflows_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let role_uuid = role_service
        .create_role(&workflows_role, admin_user_uuid)
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

    // Test that user with workflows:read cannot access dashboard
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/meta/dashboard")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User with workflows:read permission should NOT be able to access dashboard"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
