#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{http::StatusCode, test};
use r_data_core_core::error::Result;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use r_data_core_test_support::clear_test_db;
use serial_test::serial;
use std::sync::Arc;

use super::common::setup_test_app;
use r_data_core_api::jwt::generate_access_token;

/// Test that user with no permissions can still authenticate
#[tokio::test]
#[serial]
async fn test_user_with_no_permissions_can_authenticate() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a user with no roles/permissions
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "noperms_user",
            email: "noperms@example.com",
            password: "password123",
            first_name: "No",
            last_name: "Perms",
            role: Some("NoPerms"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
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

    // Test that user can authenticate (token is valid)
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with no permissions should be able to authenticate and access permissions endpoint"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that user with no permissions gets empty `allowed_routes`
#[tokio::test]
#[serial]
async fn test_user_with_no_permissions_gets_empty_allowed_routes() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a user with no roles/permissions
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "noperms_user2",
            email: "noperms2@example.com",
            password: "password123",
            first_name: "No",
            last_name: "Perms",
            role: Some("NoPerms"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
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

    // Get user permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let allowed_routes = body
        .get("allowed_routes")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    assert!(
        allowed_routes.is_empty(),
        "User with no permissions should have empty allowed_routes. Got: {allowed_routes:?}"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that user with no permissions gets empty permissions array
#[tokio::test]
#[serial]
async fn test_user_with_no_permissions_gets_empty_permissions() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a user with no roles/permissions
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "noperms_user3",
            email: "noperms3@example.com",
            password: "password123",
            first_name: "No",
            last_name: "Perms",
            role: Some("NoPerms"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
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

    // Get user permissions
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body
        .get("permissions")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    assert!(
        permissions.is_empty(),
        "User with no permissions should have empty permissions array. Got: {permissions:?}"
    );

    // Verify is_super_admin is false
    let is_super_admin = body
        .get("is_super_admin")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    assert!(
        !is_super_admin,
        "User with no permissions should not be super_admin"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that user with no permissions can access user info endpoint
#[tokio::test]
#[serial]
async fn test_user_with_no_permissions_can_access_user_info() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;

    // Create a user with no roles/permissions
    let user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = user_repo
        .create_admin_user(&CreateAdminUserParams {
            username: "noperms_user4",
            email: "noperms4@example.com",
            password: "password123",
            first_name: "No",
            last_name: "Perms",
            role: Some("NoPerms"),
            is_active: true,
            creator_uuid: admin_user_uuid,
        })
        .await?;

    // Update user to not be super_admin
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

    // Test that user can access their own user info (if such endpoint exists)
    // For now, we'll test that they can access the permissions endpoint which returns user info
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/auth/permissions")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User with no permissions should be able to access permissions endpoint to get their info"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
