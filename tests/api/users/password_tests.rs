#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{http::StatusCode, test};
use r_data_core_core::error::Result;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::clear_test_db;
use serial_test::serial;
use std::sync::Arc;

use super::common::{get_auth_token, setup_test_app};

/// Test that password update actually changes the password
#[serial]
#[tokio::test]
async fn test_password_update() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;
    let admin_token = get_auth_token(&app, &pool).await;

    // Create a test user with initial password
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "password_test_user",
        email: "password_test@example.com",
        password: "initial_password123",
        first_name: "Password",
        last_name: "Test",
        role: None,
        is_active: true,
        creator_uuid: admin_user_uuid,
    };
    let user_uuid = repo.create_admin_user(&params).await?;

    // Verify login works with initial password
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_test_user",
            "password": "initial_password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Login should work with initial password"
    );

    // Update user password via PUT endpoint
    let update_req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_token}")))
        .set_json(serde_json::json!({
            "password": "new_password456"
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Password update should succeed"
    );

    // Verify login fails with old password
    let old_password_login = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_test_user",
            "password": "initial_password123"
        }))
        .to_request();

    let resp = test::call_service(&app, old_password_login).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Login should fail with old password"
    );

    // Verify login works with new password
    let new_password_login = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_test_user",
            "password": "new_password456"
        }))
        .to_request();

    let resp = test::call_service(&app, new_password_login).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Login should work with new password"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}

/// Test that password is preserved when not provided in update request
#[serial]
#[tokio::test]
async fn test_password_preserved_when_not_provided() -> Result<()> {
    let (app, pool, admin_user_uuid) = setup_test_app().await?;
    let admin_token = get_auth_token(&app, &pool).await;

    // Create a test user with initial password
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "password_preserve_user",
        email: "password_preserve@example.com",
        password: "original_password123",
        first_name: "Password",
        last_name: "Preserve",
        role: None,
        is_active: true,
        creator_uuid: admin_user_uuid,
    };
    let user_uuid = repo.create_admin_user(&params).await?;

    // Verify login works with original password
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_preserve_user",
            "password": "original_password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Login should work with original password"
    );

    // Update user WITHOUT providing password (only update email)
    let update_req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {admin_token}")))
        .set_json(serde_json::json!({
            "email": "updated_email@example.com"
            // Note: password is NOT provided
        }))
        .to_request();

    let resp = test::call_service(&app, update_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "User update should succeed without password"
    );

    // Verify email was updated
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["email"], "updated_email@example.com",
        "Email should be updated"
    );

    // Verify login still works with ORIGINAL password (password should be preserved)
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_preserve_user",
            "password": "original_password123"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Login should still work with original password after update without password"
    );

    // Verify login fails with wrong password
    let wrong_login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "password_preserve_user",
            "password": "wrong_password"
        }))
        .to_request();

    let resp = test::call_service(&app, wrong_login_req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Login should fail with wrong password"
    );

    clear_test_db(&pool.pool).await?;
    Ok(())
}
