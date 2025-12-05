#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{http::StatusCode, test, web, App};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::permission_scheme::{
    AccessLevel, Permission, PermissionScheme, PermissionType, ResourceNamespace,
};
use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository};
use r_data_core_services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, PermissionSchemeService,
};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, setup_test_db, test_queue_client_async,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};

async fn setup_test_app() -> Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    sqlx::PgPool,
    Uuid, // user_uuid
)> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let user_uuid = create_test_admin_user(&pool).await?;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = WorkflowService::new(Arc::new(wf_adapter));

    let api_state = ApiState {
        db_pool: pool.clone(),
        api_config: r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        },
        permission_scheme_service: PermissionSchemeService::new(
            pool.clone(),
            cache_manager.clone(),
            Some(3600),
        ),
        cache_manager: cache_manager.clone(),
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
        queue: test_queue_client_async().await,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    Ok((app, pool, user_uuid))
}

async fn get_auth_token(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    pool: &sqlx::PgPool,
) -> String {
    // Get the test admin user that was created (super_admin = true)
    let username: String = sqlx::query_scalar(
        "SELECT username FROM admin_users WHERE super_admin = true ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(pool)
    .await
    .expect("Test admin user should exist");

    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": "adminadmin"
        }))
        .to_request();

    let resp = test::call_service(app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    body["data"]["access_token"].as_str().unwrap().to_string()
}

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
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
async fn test_get_user_schemes() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/users/{user_uuid}/schemes"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_assign_schemes_to_user() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // Create a permission scheme
    let scheme_service = r_data_core_services::PermissionSchemeService::new(
        pool.clone(),
        Arc::new(CacheManager::new(CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        })),
        Some(3600),
    );

    let mut scheme = PermissionScheme::new("Test Scheme".to_string());
    scheme.description = Some("Test description".to_string());
    let scheme_uuid = scheme_service
        .create_scheme(&scheme, user_uuid)
        .await
        .unwrap();

    let assign_req = vec![scheme_uuid.to_string()];

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}/schemes"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&assign_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn test_super_admin_has_all_permissions() {
    let (app, pool, _user_uuid) = setup_test_app().await.unwrap();

    // Create a super admin user
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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

    // Create a permission scheme with super_admin flag
    let scheme_service = PermissionSchemeService::new(
        pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut scheme = PermissionScheme::new("Super Admin Scheme".to_string());
    scheme.super_admin = true;
    let scheme_uuid = scheme_service
        .create_scheme(&scheme, user_uuid)
        .await
        .unwrap();

    // Create a regular user and assign the super admin scheme
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
    repo.update_user_schemes(regular_user_uuid, &[scheme_uuid])
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

    // Create a permission scheme with super_admin flag set to true
    let scheme_service = PermissionSchemeService::new(
        pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut scheme = PermissionScheme::new("Super Admin Scheme Flag Test".to_string());
    scheme.super_admin = true;
    scheme.description = Some("Test scheme with super_admin flag".to_string());
    let scheme_uuid = scheme_service
        .create_scheme(&scheme, user_uuid)
        .await
        .unwrap();

    // Verify the scheme was created with super_admin flag
    let created_scheme = scheme_service
        .get_scheme(scheme_uuid)
        .await
        .unwrap()
        .unwrap();
    assert!(
        created_scheme.super_admin,
        "Scheme should have super_admin flag set to true"
    );

    // Create a regular user (not super_admin) and assign the super admin scheme
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
    repo.update_user_schemes(regular_user_uuid, &[scheme_uuid])
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

#[serial]
#[tokio::test]
async fn test_user_management_permissions() {
    let (app, pool, admin_user_uuid) = setup_test_app().await.unwrap();
    let admin_token = get_auth_token(&app, &pool).await;

    // Create a permission scheme for regular users (no admin permissions)
    let scheme_service = PermissionSchemeService::new(
        pool.clone(),
        Arc::new(CacheManager::new(CacheConfig::default())),
        None,
    );
    let mut regular_scheme = PermissionScheme::new("Regular User Scheme".to_string());
    regular_scheme.role_permissions.insert(
        "User".to_string(),
        vec![Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        }],
    );
    let regular_scheme_uuid = scheme_service
        .create_scheme(&regular_scheme, admin_user_uuid)
        .await
        .unwrap();

    // Create a regular user (not super_admin, with limited permissions)
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
    repo.update_user_schemes(regular_user_uuid, &[regular_scheme_uuid])
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

    // Regular user should NOT be able to create users (no PermissionSchemes:Admin permission)
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

    // Create an admin user (with PermissionSchemes:Admin permission)
    let mut admin_scheme = PermissionScheme::new("Admin Scheme".to_string());
    admin_scheme.role_permissions.insert(
        "Admin".to_string(),
        vec![Permission {
            resource_type: ResourceNamespace::PermissionSchemes,
            permission_type: PermissionType::Admin,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        }],
    );
    let admin_scheme_uuid = scheme_service
        .create_scheme(&admin_scheme, admin_user_uuid)
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
    repo.update_user_schemes(admin_user_uuid, &[admin_scheme_uuid])
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

#[serial]
#[tokio::test]
async fn test_email_uniqueness() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();

    // Create first user
    let repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
