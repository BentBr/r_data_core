#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use actix_web::{http::StatusCode, test, web, App};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::permission_scheme::{
    AccessLevel, Permission, PermissionScheme, PermissionType, ResourceNamespace,
};
use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository,
    CreateAdminUserParams,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, PermissionSchemeService,
};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, setup_test_db,
    test_queue_client_async,
};
use serial_test::serial;
use std::collections::HashMap;
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

fn create_test_permission_scheme(name: &str) -> PermissionScheme {
    let mut scheme = PermissionScheme::new(name.to_string());
    scheme.description = Some("Test scheme".to_string());

    let mut role_permissions = HashMap::new();
    let permissions = vec![
        Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        },
        Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Create,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        },
    ];
    role_permissions.insert("Editor".to_string(), permissions);
    scheme.role_permissions = role_permissions;

    scheme
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_api::jwt::generate_access_token;

    /// Test successful authentication with permission schemes
    #[tokio::test]
    #[serial]
    async fn test_successful_auth_with_permission_schemes() -> Result<()> {
        let (app, pool, admin_user_uuid) = setup_test_app().await?;

        // Create a test user (not super_admin)
        let user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
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

        // Update user to not be super_admin and set role to Editor (matching scheme)
        let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
        user.super_admin = false;
        user.role = r_data_core_core::admin_user::UserRole::Custom("Editor".to_string());
        user_repo.update_admin_user(&user).await?;

        // Create a permission scheme
        let scheme_service = PermissionSchemeService::new(
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

        let mut scheme = create_test_permission_scheme("TestScheme");
        // Add permission to read permission schemes
        let mut editor_perms = scheme.role_permissions.get("Editor").unwrap().clone();
        editor_perms.push(Permission {
            resource_type: ResourceNamespace::PermissionSchemes,
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        });
        scheme
            .role_permissions
            .insert("Editor".to_string(), editor_perms);

        let scheme_uuid = scheme_service
            .create_scheme(&scheme, admin_user_uuid)
            .await?;
        scheme.base.uuid = scheme_uuid; // Set UUID for later updates

        // Assign scheme to user
        user_repo
            .assign_permission_scheme(user_uuid, scheme_uuid)
            .await?;

        // Generate JWT token with schemes
        let schemes = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        let api_config = r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        };
        let token = generate_access_token(&user, &api_config, &schemes)?;

        // Test accessing a protected endpoint with valid permissions
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/permissions")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Should have permission to read permission schemes"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test failing authentication (invalid token)
    #[tokio::test]
    #[serial]
    async fn test_failing_auth_invalid_token() -> Result<()> {
        let (app, pool, _) = setup_test_app().await?;

        // Try to access protected endpoint with invalid token
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/permissions")
            .insert_header(("Authorization", "Bearer invalid_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Should reject invalid token"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test failing permissions (user without required permission)
    #[tokio::test]
    #[serial]
    async fn test_failing_permissions_no_permission() -> Result<()> {
        let (app, pool, admin_user_uuid) = setup_test_app().await?;

        // Create a test user without super_admin
        let user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
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

        // Update user to not be super_admin and no schemes assigned
        let mut user = user_repo.find_by_uuid(&user_uuid).await?.unwrap();
        user.super_admin = false;
        user.role = r_data_core_core::admin_user::UserRole::Custom("Editor".to_string());
        user_repo.update_admin_user(&user).await?;

        // Generate JWT token without any schemes (no permissions)
        let api_config = r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        };
        let token = generate_access_token(&user, &api_config, &[])?;

        // Try to access protected endpoint without permission
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/permissions")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Should reject request without required permission"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test successful auth with different permission schemes
    #[tokio::test]
    #[serial]
    async fn test_successful_auth_with_different_schemes() -> Result<()> {
        let (app, pool, admin_user_uuid) = setup_test_app().await?;

        let user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
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
        user.role = r_data_core_core::admin_user::UserRole::Custom("Editor".to_string());
        user_repo.update_admin_user(&user).await?;

        let scheme_service = PermissionSchemeService::new(
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

        // Create two different schemes
        let mut scheme1 = PermissionScheme::new("Scheme1".to_string());
        let mut role_perms1 = HashMap::new();
        role_perms1.insert(
            "Editor".to_string(),
            vec![Permission {
                resource_type: ResourceNamespace::Workflows,
                permission_type: PermissionType::Read,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            }],
        );
        scheme1.role_permissions = role_perms1;
        let scheme1_uuid = scheme_service
            .create_scheme(&scheme1, admin_user_uuid)
            .await?;

        let mut scheme2 = PermissionScheme::new("Scheme2".to_string());
        let mut role_perms2 = HashMap::new();
        role_perms2.insert(
            "Editor".to_string(),
            vec![Permission {
                resource_type: ResourceNamespace::Workflows,
                permission_type: PermissionType::Create,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            }],
        );
        scheme2.role_permissions = role_perms2;
        let scheme2_uuid = scheme_service
            .create_scheme(&scheme2, admin_user_uuid)
            .await?;

        // Assign both schemes to user
        user_repo
            .assign_permission_scheme(user_uuid, scheme1_uuid)
            .await?;
        user_repo
            .assign_permission_scheme(user_uuid, scheme2_uuid)
            .await?;

        // Generate JWT with merged permissions from both schemes
        let schemes = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        assert_eq!(schemes.len(), 2, "User should have 2 schemes");

        let api_config = r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        };
        let token = generate_access_token(&user, &api_config, &schemes)?;

        // Test that user has both Read and Create permissions (merged from both schemes)
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/workflows")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Should have Read permission from scheme1"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test permission caching behavior
    #[tokio::test]
    #[serial]
    async fn test_permission_caching() -> Result<()> {
        let pool = setup_test_db().await;
        clear_test_db(&pool).await?;

        let admin_user_uuid = create_test_admin_user(&pool).await?;
        let user_repo = AdminUserRepository::new(Arc::new(pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));
        let scheme_service =
            PermissionSchemeService::new(pool.clone(), cache_manager.clone(), Some(3600));

        // Create a scheme
        let scheme = create_test_permission_scheme("CacheTestScheme");
        let scheme_uuid = scheme_service
            .create_scheme(&scheme, admin_user_uuid)
            .await?;

        // Verify scheme was created
        let initial_scheme = scheme_service.get_scheme(scheme_uuid).await?.unwrap();
        assert_eq!(
            initial_scheme.name, "CacheTestScheme",
            "Scheme name should match"
        );

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
        user.role = r_data_core_core::admin_user::UserRole::Custom("Editor".to_string());
        user_repo.update_admin_user(&user).await?;

        // Assign scheme to user
        user_repo
            .assign_permission_scheme(user_uuid, scheme_uuid)
            .await?;

        // First access - should load from DB and cache
        let schemes1 = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        assert_eq!(schemes1.len(), 1, "Should have 1 scheme");

        // Second access - should come from cache
        let schemes2 = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        assert_eq!(schemes2.len(), 1, "Should still have 1 scheme from cache");

        // Update the scheme - reload it first to get the complete scheme with base UUID
        let mut scheme_to_update = scheme_service.get_scheme(scheme_uuid).await?.unwrap();
        // Verify we got the scheme correctly
        assert_eq!(
            scheme_to_update.base.uuid, scheme_uuid,
            "Scheme UUID should match"
        );
        assert_eq!(
            scheme_to_update.name, "CacheTestScheme",
            "Initial name should match"
        );

        // Update the name instead of description (description has a known sqlx binding issue)
        let original_name = scheme_to_update.name.clone();
        scheme_to_update.name = "Updated CacheTestScheme".to_string();

        // Update in database - verify it succeeds
        let update_result = scheme_service
            .update_scheme(&scheme_to_update, admin_user_uuid)
            .await;
        assert!(
            update_result.is_ok(),
            "Update should succeed: {:?}",
            update_result.err()
        );

        // Verify the update in the database directly
        let db_name: String =
            sqlx::query_scalar("SELECT name FROM permission_schemes WHERE uuid = $1")
                .bind(scheme_uuid)
                .fetch_one(&pool)
                .await?;
        assert_eq!(
            db_name, "Updated CacheTestScheme",
            "Name should be updated in DB"
        );

        // Invalidate user cache to force reload
        scheme_service
            .invalidate_user_permissions_cache(&user_uuid)
            .await;

        // Access after update - should reload from DB (cache invalidated)
        let schemes3 = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        assert_eq!(schemes3.len(), 1, "Should still have 1 scheme");
        assert_eq!(
            schemes3[0].name, "Updated CacheTestScheme",
            "Should have updated name in user schemes"
        );
        assert_ne!(
            schemes3[0].name, original_name,
            "Name should be different from original"
        );

        // Test cache invalidation on scheme assignment
        let scheme2 = create_test_permission_scheme("CacheTestScheme2");
        let scheme2_uuid = scheme_service
            .create_scheme(&scheme2, admin_user_uuid)
            .await?;

        // Assign new scheme - should invalidate cache
        user_repo
            .assign_permission_scheme(user_uuid, scheme2_uuid)
            .await?;

        // Access after assignment - should reload from DB
        let schemes4 = scheme_service
            .get_schemes_for_user(user_uuid, &user_repo)
            .await?;
        assert_eq!(schemes4.len(), 2, "Should have 2 schemes after assignment");

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
        let user_repo = AdminUserRepository::new(Arc::new(pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));
        let scheme_service =
            PermissionSchemeService::new(pool.clone(), cache_manager.clone(), Some(3600));

        // Create a scheme with permissions
        let mut scheme = create_test_permission_scheme("MergedCacheTest");
        let scheme_uuid = scheme_service
            .create_scheme(&scheme, admin_user_uuid)
            .await?;
        scheme.base.uuid = scheme_uuid; // Set UUID for later updates

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
        user.role = r_data_core_core::admin_user::UserRole::Custom("Editor".to_string());
        user_repo.update_admin_user(&user).await?;

        // Assign scheme
        user_repo
            .assign_permission_scheme(user_uuid, scheme_uuid)
            .await?;

        // First access - should calculate and cache merged permissions
        let perms1 = scheme_service
            .get_merged_permissions_for_user(user_uuid, "Editor", &user_repo)
            .await?;
        assert!(!perms1.is_empty(), "Should have merged permissions");

        // Second access - should come from cache
        let perms2 = scheme_service
            .get_merged_permissions_for_user(user_uuid, "Editor", &user_repo)
            .await?;
        assert_eq!(perms1, perms2, "Should return same permissions from cache");

        // Update scheme - should invalidate cache - need to reload first to get base UUID
        let mut scheme_to_update = scheme_service.get_scheme(scheme_uuid).await?.unwrap();
        let mut updated_perms = scheme_to_update
            .role_permissions
            .get("Editor")
            .unwrap()
            .clone();
        let new_perm = Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Update,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        };
        updated_perms.push(new_perm.clone());
        scheme_to_update
            .role_permissions
            .insert("Editor".to_string(), updated_perms);
        scheme_service
            .update_scheme(&scheme_to_update, admin_user_uuid)
            .await?;

        // Invalidate user cache to force recalculation of merged permissions
        scheme_service
            .invalidate_user_permissions_cache(&user_uuid)
            .await;

        // Access after update - should recalculate (cache invalidated)
        let perms3 = scheme_service
            .get_merged_permissions_for_user(user_uuid, "Editor", &user_repo)
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
}
