#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{http::StatusCode, test};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use super::common::{get_auth_token, setup_test_app};

#[serial]
#[tokio::test]
async fn test_list_users() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_list_users_sorted_by_roles() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // Ensure at least one user-role link exists
    let admin_uuid: uuid::Uuid =
        sqlx::query_scalar("SELECT uuid FROM admin_users ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&pool.pool)
            .await
            .expect("admin user exists");
    // Ensure there is at least one role to relate; create a simple one if missing
    let role_uuid: uuid::Uuid = if let Some(existing) = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT uuid FROM roles ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&pool.pool)
    .await
    .expect("role lookup")
    {
        existing
    } else {
        sqlx::query_scalar("INSERT INTO roles (name, description, permissions, created_by) VALUES ($1, $2, $3, $4) RETURNING uuid")
            .bind("test-role")
            .bind("role for sorting test")
            .bind(serde_json::json!([]))
            .bind(admin_uuid)
            .fetch_one(&pool.pool)
            .await
            .expect("role insert")
    };
    let _ = sqlx::query!(
        "INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        admin_uuid,
        role_uuid
    )
    .execute(&pool.pool)
    .await;

    let req = test::TestRequest::get()
        .uri("/admin/api/v1/users?per_page=50&sort_by=roles&sort_order=asc")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "sorting by roles should be allowed"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_get_user() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["uuid"], user_uuid.to_string());
}

#[serial]
#[tokio::test]
async fn test_create_user() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let create_req = serde_json::json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "password123",
        "first_name": "Test",
        "last_name": "User",
        "is_active": true,
        "super_admin": false
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&create_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["username"], "testuser");
    assert_eq!(body["data"]["email"], "test@example.com");
}

#[serial]
#[tokio::test]
async fn test_update_user() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let update_req = serde_json::json!({
        "email": "updated@example.com",
        "first_name": "Updated",
        "last_name": "Name",
        "super_admin": true
    });

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&update_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["email"], "updated@example.com");
    assert_eq!(body["data"]["super_admin"], true);
}

#[serial]
#[tokio::test]
async fn test_delete_user() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // Create a user to delete
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: "todelete",
        email: "todelete@example.com",
        password: "password123",
        first_name: "To",
        last_name: "Delete",
        role: None,
        is_active: true,
        creator_uuid: Uuid::now_v7(),
    };
    let delete_user_uuid = repo.create_admin_user(&params).await.unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/admin/api/v1/users/{delete_user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_get_user_roles() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/users/{user_uuid}/roles"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_assign_roles_to_user() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // Create a role
    let role_service = r_data_core_services::RoleService::new(
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

    let mut role = Role::new("Test Scheme".to_string());
    role.description = Some("Test description".to_string());
    let role_uuid = role_service.create_role(&role, user_uuid).await.unwrap();

    let assign_req = vec![role_uuid.to_string()];

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}/roles"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&assign_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_email_uniqueness() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();

    // Create first user
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let params1 = r_data_core_persistence::CreateAdminUserParams {
        username: "user1",
        email: "test@example.com",
        password: "password123",
        first_name: "User",
        last_name: "One",
        role: None,
        is_active: true,
        creator_uuid: user_uuid,
    };
    let _user1_uuid = repo.create_admin_user(&params1).await.unwrap();

    // Try to create second user with same email
    let params2 = r_data_core_persistence::CreateAdminUserParams {
        username: "user2",
        email: "test@example.com", // Same email
        password: "password123",
        first_name: "User",
        last_name: "Two",
        role: None,
        is_active: true,
        creator_uuid: user_uuid,
    };

    // Should fail with conflict - repository will return an error
    let result = repo.create_admin_user(&params2).await;
    assert!(
        result.is_err(),
        "Creating user with duplicate email should fail"
    );

    // Also test via API endpoint - use super admin token
    let token = get_auth_token(&app, &pool).await;

    // Try to create user with duplicate email via API
    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/users")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "username": "user3",
            "email": "test@example.com", // Duplicate email
            "password": "password123",
            "first_name": "User",
            "last_name": "Three"
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
