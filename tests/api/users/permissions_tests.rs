#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions use impl Service from setup_test_app() across awaits

use actix_web::{http::StatusCode, test};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_services::RoleService;
use r_data_core_test_support::clear_test_db;
use serial_test::serial;
use std::sync::Arc;

use super::common::{get_auth_token, setup_test_app};

#[serial]
#[tokio::test]
async fn test_user_management_permissions() {
    let (app, pool, admin_user_uuid) = setup_test_app().await.unwrap();
    let admin_token = get_auth_token(&app, &pool).await;

    // Create a role for regular users (no admin permissions)
    let role_service = RoleService::new(
        pool.pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut regular_role = Role::new("Regular User Scheme".to_string());
    regular_role.permissions = vec![Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];
    let regular_role_uuid = role_service
        .create_role(&regular_role, admin_user_uuid)
        .await
        .unwrap();

    // Create a regular user (not super_admin, with limited permissions)
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let regular_user_params = r_data_core_persistence::CreateAdminUserParams {
        username: "regular_user",
        email: "regular@example.com",
        password: "password123",
        first_name: "Regular",
        last_name: "User",
        role: Some("User"),
        is_active: true,
        creator_uuid: admin_user_uuid,
    };
    let regular_user_uuid = repo.create_admin_user(&regular_user_params).await.unwrap();
    repo.update_user_roles(regular_user_uuid, &[regular_role_uuid])
        .await
        .unwrap();

    // Login as regular user
    let regular_login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "regular_user",
            "password": "password123"
        }))
        .to_request();
    let regular_resp = test::call_service(&app, regular_login_req).await;
    assert_eq!(regular_resp.status(), StatusCode::OK);
    let regular_body: serde_json::Value = test::read_body_json(regular_resp).await;
    let regular_token = regular_body["data"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Regular user should NOT be able to create users (no Users:Create permission)
    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {regular_token}")))
        .set_json(serde_json::json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "password": "password123",
            "first_name": "New",
            "last_name": "User",
            "is_active": true
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(
        create_resp.status(),
        StatusCode::FORBIDDEN,
        "Regular user should not be able to create users"
    );

    // Create an admin user (with Users:Admin permission)
    let mut admin_role = Role::new("Admin Scheme".to_string());
    admin_role.permissions = vec![Permission {
        resource_type: ResourceNamespace::Users,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];
    let admin_role_uuid = role_service
        .create_role(&admin_role, admin_user_uuid)
        .await
        .unwrap();

    let admin_user_params = r_data_core_persistence::CreateAdminUserParams {
        username: "admin_user",
        email: "admin_user@example.com",
        password: "password123",
        first_name: "Admin",
        last_name: "User",
        role: Some("Admin"),
        is_active: true,
        creator_uuid: admin_user_uuid,
    };
    let admin_user_uuid = repo.create_admin_user(&admin_user_params).await.unwrap();
    repo.update_user_roles(admin_user_uuid, &[admin_role_uuid])
        .await
        .unwrap();

    // Login as admin user
    let admin_user_login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "admin_user",
            "password": "password123"
        }))
        .to_request();
    let admin_user_resp = test::call_service(&app, admin_user_login_req).await;
    assert_eq!(admin_user_resp.status(), StatusCode::OK);
    let admin_user_body: serde_json::Value = test::read_body_json(admin_user_resp).await;
    let admin_user_token = admin_user_body["data"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Admin user SHOULD be able to create regular users
    let admin_create_req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {admin_user_token}")))
        .set_json(serde_json::json!({
            "username": "newuser_by_admin",
            "email": "newuser_by_admin@example.com",
            "password": "password123",
            "first_name": "New",
            "last_name": "User",
            "is_active": true
        }))
        .to_request();
    let admin_create_resp = test::call_service(&app, admin_create_req).await;
    assert_eq!(
        admin_create_resp.status(),
        StatusCode::CREATED,
        "Admin user should be able to create users"
    );

    // Admin user should be able to update regular users
    let admin_update_regular_req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{regular_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_user_token}")))
        .set_json(serde_json::json!({
            "email": "updated_by_admin@example.com"
        }))
        .to_request();
    let admin_update_regular_resp = test::call_service(&app, admin_update_regular_req).await;
    assert_eq!(
        admin_update_regular_resp.status(),
        StatusCode::OK,
        "Admin should be able to update regular users"
    );

    // Super admin (from setup) SHOULD be able to update admin users
    let super_admin_update_req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{admin_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_token}")))
        .set_json(serde_json::json!({
            "email": "updated_by_superadmin@example.com"
        }))
        .to_request();
    let super_admin_update_resp = test::call_service(&app, super_admin_update_req).await;
    assert_eq!(
        super_admin_update_resp.status(),
        StatusCode::OK,
        "Super admin should be able to update admin users"
    );
}
/// Test Users namespace permissions
#[serial]
#[tokio::test]
async fn test_users_namespace_permissions() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
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

    // Create a user with Users:Read permission
    let read_user_uuid = user_repo
        .create_admin_user(&r_data_core_persistence::CreateAdminUserParams {
            username: "users_read_user",
            email: "users_read@example.com",
            password: "password123",
            first_name: "Read",
            last_name: "User",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut read_user = user_repo.find_by_uuid(&read_user_uuid).await?.unwrap();
    read_user.super_admin = false;
    user_repo.update_admin_user(&read_user).await?;

    let mut read_role = Role::new("UsersReadRole".to_string());
    read_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Users,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let read_role_uuid = role_service
        .create_role(&read_role, admin_user_uuid)
        .await?;
    user_repo
        .assign_role(read_user_uuid, read_role_uuid)
        .await?;

    let roles = role_service
        .get_roles_for_user(read_user_uuid, &user_repo)
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
    let read_token =
        r_data_core_core::admin_jwt::generate_access_token(&read_user, &api_config, &roles)?;

    // User with Users:Read should be able to list users
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {read_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with Users:Read should be able to list users"
    );

    // User with Users:Read should be able to get a specific user
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/users/{read_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {read_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with Users:Read should be able to get a user"
    );

    // User with Users:Read should NOT be able to create users
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {read_token}")))
        .set_json(serde_json::json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "password": "password123",
            "first_name": "New",
            "last_name": "User"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User with Users:Read should NOT be able to create users"
    );

    // Create a user with Users:Create permission
    let create_user_uuid = user_repo
        .create_admin_user(&r_data_core_persistence::CreateAdminUserParams {
            username: "users_create_user",
            email: "users_create@example.com",
            password: "password123",
            first_name: "Create",
            last_name: "User",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut create_user = user_repo.find_by_uuid(&create_user_uuid).await?.unwrap();
    create_user.super_admin = false;
    user_repo.update_admin_user(&create_user).await?;

    let mut create_role = Role::new("UsersCreateRole".to_string());
    create_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Users,
        permission_type: PermissionType::Create,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let create_role_uuid = role_service
        .create_role(&create_role, admin_user_uuid)
        .await?;
    user_repo
        .assign_role(create_user_uuid, create_role_uuid)
        .await?;

    let roles = role_service
        .get_roles_for_user(create_user_uuid, &user_repo)
        .await?;
    let create_token =
        r_data_core_core::admin_jwt::generate_access_token(&create_user, &api_config, &roles)?;

    // User with Users:Create should be able to create users
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {create_token}")))
        .set_json(serde_json::json!({
            "username": "created_user",
            "email": "created@example.com",
            "password": "password123",
            "first_name": "Created",
            "last_name": "User"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "User with Users:Create should be able to create users"
    );

    // Create a user with Users:Admin permission
    let admin_user_uuid2 = user_repo
        .create_admin_user(&r_data_core_persistence::CreateAdminUserParams {
            username: "users_admin_user",
            email: "users_admin@example.com",
            password: "password123",
            first_name: "Admin",
            last_name: "User",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut admin_user2 = user_repo.find_by_uuid(&admin_user_uuid2).await?.unwrap();
    admin_user2.super_admin = false;
    user_repo.update_admin_user(&admin_user2).await?;

    let mut admin_role = Role::new("UsersAdminRole".to_string());
    admin_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Users,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let admin_role_uuid = role_service
        .create_role(&admin_role, admin_user_uuid)
        .await?;
    user_repo
        .assign_role(admin_user_uuid2, admin_role_uuid)
        .await?;

    let roles = role_service
        .get_roles_for_user(admin_user_uuid2, &user_repo)
        .await?;
    let admin_token2 =
        r_data_core_core::admin_jwt::generate_access_token(&admin_user2, &api_config, &roles)?;

    // User with Users:Admin should be able to do all operations
    // Read
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {admin_token2}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with Users:Admin should be able to list users"
    );

    // Create
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {admin_token2}")))
        .set_json(serde_json::json!({
            "username": "admin_created_user",
            "email": "admin_created@example.com",
            "password": "password123",
            "first_name": "Admin",
            "last_name": "Created"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "User with Users:Admin should be able to create users"
    );

    // Update
    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{read_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_token2}")))
        .set_json(serde_json::json!({
            "email": "updated_by_admin@example.com"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with Users:Admin should be able to update users"
    );

    // Delete
    let delete_user_uuid = user_repo
        .create_admin_user(&r_data_core_persistence::CreateAdminUserParams {
            username: "to_delete_user",
            email: "todelete@example.com",
            password: "password123",
            first_name: "To",
            last_name: "Delete",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let req = test::TestRequest::delete()
        .uri(&format!("/admin/api/v1/users/{delete_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_token2}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with Users:Admin should be able to delete users"
    );

    // Test that Roles:Admin does NOT grant access to Users endpoints
    let roles_admin_user_uuid = user_repo
        .create_admin_user(&r_data_core_persistence::CreateAdminUserParams {
            username: "roles_admin_user",
            email: "roles_admin@example.com",
            password: "password123",
            first_name: "Roles",
            last_name: "Admin",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    let mut roles_admin_user = user_repo
        .find_by_uuid(&roles_admin_user_uuid)
        .await?
        .unwrap();
    roles_admin_user.super_admin = false;
    user_repo.update_admin_user(&roles_admin_user).await?;

    let mut roles_admin_role = Role::new("RolesAdminRole".to_string());
    roles_admin_role.permissions.push(Permission {
        resource_type: ResourceNamespace::Roles,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    });

    let roles_admin_role_uuid = role_service
        .create_role(&roles_admin_role, admin_user_uuid)
        .await?;
    user_repo
        .assign_role(roles_admin_user_uuid, roles_admin_role_uuid)
        .await?;

    let roles = role_service
        .get_roles_for_user(roles_admin_user_uuid, &user_repo)
        .await?;
    let roles_admin_token =
        r_data_core_core::admin_jwt::generate_access_token(&roles_admin_user, &api_config, &roles)?;

    // User with Roles:Admin should NOT be able to list users
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {roles_admin_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "User with Roles:Admin should NOT be able to access Users endpoints"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
