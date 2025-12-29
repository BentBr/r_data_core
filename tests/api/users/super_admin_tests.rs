#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{http::StatusCode, test};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_services::RoleService;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use super::common::setup_test_app;

#[serial]
#[tokio::test]
async fn test_super_admin_has_all_permissions() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();

    // Create a super admin user
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "superadmin",
        email: "superadmin@example.com",
        password: "password123",
        first_name: "Super",
        last_name: "Admin",
        role: None,
        is_active: true,
        creator_uuid: Uuid::now_v7(),
    };
    let super_admin_uuid = repo.create_admin_user(&params).await.unwrap();

    // Update to super_admin
    let mut user = repo.find_by_uuid(&super_admin_uuid).await.unwrap().unwrap();
    user.super_admin = true;
    repo.update_admin_user(&user).await.unwrap();

    // Login as super admin
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "superadmin",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    // Get permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["is_super_admin"], true);
    assert!(!body["data"]["allowed_routes"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[serial]
#[tokio::test]
async fn test_super_admin_has_all_permissions_from_user_flag() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();

    // Create a user and then set super_admin flag
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "superadmin_flag",
        email: "superadmin_flag@example.com",
        password: "password123",
        first_name: "Super",
        last_name: "Admin",
        role: None,
        is_active: true,
        creator_uuid: Uuid::now_v7(),
    };
    let super_admin_uuid = repo.create_admin_user(&params).await.unwrap();

    // Set super_admin flag
    let mut user = repo.find_by_uuid(&super_admin_uuid).await.unwrap().unwrap();
    user.super_admin = true;
    repo.update_admin_user(&user).await.unwrap();

    // Login as super admin
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "superadmin_flag",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    // Get permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["is_super_admin"], true);
    // Super admin should have access to all routes
    let allowed_routes = body["data"]["allowed_routes"].as_array().unwrap();
    assert!(allowed_routes.len() >= 7); // Should have access to all defined routes
}

#[serial]
#[tokio::test]
async fn test_super_admin_has_all_permissions_from_permission() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();

    // Create a role with super_admin flag
    let role_service = RoleService::new(
        pool.pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut role = Role::new("Super Admin Scheme".to_string());
    role.super_admin = true;
    let role_uuid = role_service.create_role(&role, user_uuid).await.unwrap();

    // Create a regular user and assign the super admin role
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "superadmin_scheme",
        email: "superadmin_scheme@example.com",
        password: "password123",
        first_name: "Super",
        last_name: "Admin",
        role: None,
        is_active: true,
        creator_uuid: user_uuid,
    };
    let regular_user_uuid = repo.create_admin_user(&params).await.unwrap();

    // Assign the super admin scheme to the user
    repo.update_user_roles(regular_user_uuid, &[role_uuid])
        .await
        .unwrap();

    // Login as the user
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "superadmin_scheme",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    // Get permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    // User with super_admin scheme should have all permissions
    assert_eq!(body["data"]["is_super_admin"], true);
    let allowed_routes = body["data"]["allowed_routes"].as_array().unwrap();
    assert!(allowed_routes.len() >= 7); // Should have access to all defined routes
}

#[serial]
#[tokio::test]
async fn test_super_admin_flag_on_scheme_grants_all_permissions() {
    // Decode JWT to verify is_super_admin is set
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use r_data_core_api::jwt::AuthUserClaims;

    let (app, pool, user_uuid) = setup_test_app().await.unwrap();

    // Create a role with super_admin flag set to true
    let role_service = RoleService::new(
        pool.pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut role = Role::new("Super Admin Scheme Flag Test".to_string());
    role.super_admin = true;
    role.description = Some("Test scheme with super_admin flag".to_string());
    let role_uuid = role_service.create_role(&role, user_uuid).await.unwrap();

    // Verify the role was created with super_admin flag
    let created_role = role_service.get_role(role_uuid).await.unwrap().unwrap();
    assert!(
        created_role.super_admin,
        "Role should have super_admin flag set to true"
    );

    // Create a regular user (not super_admin) and assign the super admin scheme
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "regular_user_scheme",
        email: "regular_user_scheme@example.com",
        password: "password123",
        first_name: "Regular",
        last_name: "User",
        role: None,
        is_active: true,
        creator_uuid: user_uuid,
    };
    let regular_user_uuid = repo.create_admin_user(&params).await.unwrap();

    // Verify user is not super_admin
    let user = repo
        .find_by_uuid(&regular_user_uuid)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !user.super_admin,
        "User should not be super_admin initially"
    );

    // Assign the super admin scheme to the user
    repo.update_user_roles(regular_user_uuid, &[role_uuid])
        .await
        .unwrap();

    // Login as the user
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "regular_user_scheme",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    let jwt_secret = "test_secret";
    let validation = Validation::default();
    let token_data = decode::<AuthUserClaims>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .unwrap();

    // User with a super_admin scheme should have is_super_admin = true in JWT
    assert!(
        token_data.claims.is_super_admin,
        "JWT should have is_super_admin = true when user has super_admin scheme"
    );

    // Get permissions endpoint should also reflect super_admin status
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["is_super_admin"], true,
        "Permissions endpoint should return is_super_admin = true"
    );
    let allowed_routes = body["data"]["allowed_routes"].as_array().unwrap();
    assert!(
        allowed_routes.len() >= 7,
        "Super admin should have access to all defined routes"
    );
}
