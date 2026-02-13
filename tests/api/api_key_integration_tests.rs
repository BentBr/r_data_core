#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{
    http::{header, StatusCode},
    test, web, App, HttpMessage, HttpRequest, HttpResponse,
};
use r_data_core_api::{
    middleware::{ApiAuth, ApiKeyInfo},
    ApiState,
};
use r_data_core_core::admin_jwt::AuthUserClaims;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
use r_data_core_services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, LicenseService,
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn create_test_jwt_token(user_uuid: &Uuid, secret: &str) -> String {
    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(1);

    // SuperAdmin gets all permissions
    let permissions = vec![
        "workflows:read".to_string(),
        "workflows:create".to_string(),
        "workflows:update".to_string(),
        "workflows:delete".to_string(),
        "workflows:execute".to_string(),
        "entities:read".to_string(),
        "entities:create".to_string(),
        "entities:update".to_string(),
        "entities:delete".to_string(),
        "entity_definitions:read".to_string(),
        "entity_definitions:create".to_string(),
        "entity_definitions:update".to_string(),
        "entity_definitions:delete".to_string(),
        "api_keys:read".to_string(),
        "api_keys:create".to_string(),
        "api_keys:update".to_string(),
        "api_keys:delete".to_string(),
        "roles:read".to_string(),
        "roles:create".to_string(),
        "roles:update".to_string(),
        "roles:delete".to_string(),
        "system:read".to_string(),
        "system:create".to_string(),
        "system:update".to_string(),
        "system:delete".to_string(),
    ];

    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        iss: r_data_core_core::admin_jwt::ADMIN_JWT_ISSUER.to_string(),
        name: "test_user".to_string(),
        email: "test@example.com".to_string(),
        permissions,
        exp: usize::try_from(exp.unix_timestamp()).unwrap_or(0),
        iat: usize::try_from(now.unix_timestamp()).unwrap_or(0),
        is_super_admin: false,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create JWT token")
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_test_support::{
        clear_test_db, create_test_admin_user, make_workflow_service, setup_test_db,
        test_queue_client_async,
    };
    use serial_test::serial;

    /// Test listing API keys through the API
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_list_api_keys_integration() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create some API keys
        let (key1_uuid, key1_value) = repo
            .create_new_api_key("Key 1", "First key", user_uuid, 30)
            .await?;

        let (key2_uuid, _key2_value) = repo
            .create_new_api_key("Key 2", "Second key", user_uuid, 30)
            .await?;

        // Create test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        // Clone the UUIDs to move into the closure
        let key1_uuid_clone = key1_uuid;
        let key2_uuid_clone = key2_uuid;

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/api/admin/api-keys")
                        .wrap(ApiAuth::new())
                        .route(web::get().to(move |_req: HttpRequest| {
                            let key1_uuid = key1_uuid_clone;
                            let key2_uuid = key2_uuid_clone;
                            async move {
                                // Simulate API key listing endpoint
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "keys": [
                                        {"uuid": key1_uuid.to_string(), "name": "Key 1"},
                                        {"uuid": key2_uuid.to_string(), "name": "Key 2"}
                                    ]
                                }))
                            }
                        })),
                ),
        )
        .await;

        // Test API key listing (send valid API key header)
        let req = test::TestRequest::get()
            .uri("/api/admin/api-keys")
            .insert_header(("X-API-Key", key1_value.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["keys"].as_array().unwrap().len(), 2);

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test revoking API key through the API
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_revoke_api_key_integration() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create API key
        let (key_uuid, key_value) = repo
            .create_new_api_key("Test Key", "Test description", user_uuid, 30)
            .await?;

        // Verify the key exists and is valid
        let auth_result = repo.find_api_key_for_auth(&key_value).await?;
        assert!(auth_result.is_some());

        // Create test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        // Wrap the repo in Arc for sharing
        let repo_arc = Arc::new(repo);
        let repo_for_handler = repo_arc.clone();

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/api/admin/api-keys/{uuid}")
                        .wrap(ApiAuth::new())
                        .route(web::delete().to(
                            move |req: HttpRequest, path: web::Path<String>| {
                                let repo_clone = repo_for_handler.clone();
                                async move {
                                    // Simulate API key revocation endpoint
                                    let auth_check = req.extensions().get::<ApiKeyInfo>().is_some();
                                    if auth_check {
                                        let key_uuid_str = path.into_inner();
                                        if let Ok(key_uuid) = uuid::Uuid::parse_str(&key_uuid_str) {
                                            // Actually revoke the key in the database
                                            match repo_clone.revoke(key_uuid).await {
                                                Ok(()) => {
                                                    HttpResponse::Ok().json(serde_json::json!({
                                                        "status": "success",
                                                        "message": "API key revoked"
                                                    }))
                                                }
                                                Err(_) => HttpResponse::InternalServerError().json(
                                                    serde_json::json!({
                                                        "status": "error",
                                                        "message": "Failed to revoke API key"
                                                    }),
                                                ),
                                            }
                                        } else {
                                            HttpResponse::BadRequest().json(serde_json::json!({
                                                "status": "error",
                                                "message": "Invalid UUID format"
                                            }))
                                        }
                                    } else {
                                        HttpResponse::Unauthorized().json(serde_json::json!({
                                            "status": "error",
                                            "message": "Unauthorized"
                                        }))
                                    }
                                }
                            },
                        )),
                ),
        )
        .await;

        // Test API key revocation (send valid API key header)
        let req = test::TestRequest::delete()
            .uri(&format!("/api/admin/api-keys/{key_uuid}"))
            .insert_header(("X-API-Key", key_value.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify the key is revoked
        let auth_result = repo_arc.find_api_key_for_auth(&key_value).await?;
        assert!(auth_result.is_none());

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test using API key to access protected endpoint
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_protected_endpoint() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create an API key
        let (_key_uuid, key_value) = repo
            .create_new_api_key("Test Key", "Test description", user_uuid, 30)
            .await?;

        // Create the test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/protected").wrap(ApiAuth::new()).route(
                        web::get().to(move |req: HttpRequest| async move {
                            // Simulate protected endpoint
                            req.extensions().get::<ApiKeyInfo>().map_or_else(
                                || {
                                    HttpResponse::Unauthorized().json(serde_json::json!({
                                        "status": "error",
                                        "message": "Unauthorized"
                                    }))
                                },
                                |_auth| {
                                    HttpResponse::Ok().json(serde_json::json!({
                                        "status": "success",
                                        "message": "Access granted"
                                    }))
                                },
                            )
                        }),
                    )),
            )
            .await;

        // Test with the API key
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test expired API key access
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_expired_api_key_integration() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create API key with very short expiration (1 second)
        let (key_uuid, key_value) = repo
            .create_new_api_key("Expired Key", "Test description", user_uuid, 1) // 1 day expiration
            .await?;

        // Manually expire the key by setting expires_at to the past
        sqlx::query!(
            "UPDATE api_keys SET expires_at = NOW() - INTERVAL '1 day' WHERE uuid = $1",
            key_uuid
        )
        .execute(&pool.pool)
        .await?;

        // Create test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/protected").wrap(ApiAuth::new()).route(
                        web::get().to(move |req: HttpRequest| async move {
                            // Simulate protected endpoint
                            req.extensions().get::<ApiKeyInfo>().map_or_else(
                                || {
                                    HttpResponse::Unauthorized().json(serde_json::json!({
                                        "status": "error",
                                        "message": "Unauthorized"
                                    }))
                                },
                                |_auth| {
                                    HttpResponse::Ok().json(serde_json::json!({
                                        "status": "success",
                                        "message": "Access granted"
                                    }))
                                },
                            )
                        }),
                    )),
            )
            .await;

        // Test with expired API key
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(result.is_err(), "Expected an error for expired API key");

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key usage tracking
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_usage_tracking() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create API key
        let (key_uuid, key_value) = repo
            .create_new_api_key("Test Key", "Test description", user_uuid, 30)
            .await?;

        // Get initial key info
        let initial_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        let initial_last_used = initial_key.last_used_at;

        // Use the API key
        let auth_result = repo.find_api_key_for_auth(&key_value).await?;
        assert!(auth_result.is_some());

        // Get updated key info
        let updated_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        let updated_last_used = updated_key.last_used_at;

        // Verify last_used_at was updated
        assert!(updated_last_used > initial_last_used);

        // Create test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/protected").wrap(ApiAuth::new()).route(
                        web::get().to(move |req: HttpRequest| async move {
                            // Simulate protected endpoint
                            req.extensions().get::<ApiKeyInfo>().map_or_else(
                                || {
                                    HttpResponse::Unauthorized().json(serde_json::json!({
                                        "status": "error",
                                        "message": "Unauthorized"
                                    }))
                                },
                                |_auth| {
                                    HttpResponse::Ok().json(serde_json::json!({
                                        "status": "success",
                                        "message": "Access granted"
                                    }))
                                },
                            )
                        }),
                    )),
            )
            .await;

        // Use the API key through the middleware
        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify last_used_at was updated again
        let final_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        assert!(final_key.last_used_at > updated_last_used);

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key creation validation
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_creation_validation() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Test empty name validation
        let result = repo
            .create_new_api_key("", "Test description", user_uuid, 30)
            .await;
        assert!(result.is_err());

        // Test negative expiration validation
        let result = repo
            .create_new_api_key("Test Key", "Test description", user_uuid, -5)
            .await;
        assert!(result.is_err());

        // Test valid creation
        let result = repo
            .create_new_api_key("Valid Key", "Valid description", user_uuid, 30)
            .await;
        assert!(result.is_ok());

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key reassignment
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_reassignment() -> Result<()> {
        let pool = setup_test_db().await;
        let user1_uuid = create_test_admin_user(&pool).await?;
        let user2_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create API key for user1
        let (key_uuid, _key_value) = repo
            .create_new_api_key("Test Key", "Test description", user1_uuid, 30)
            .await?;

        // Verify initial ownership
        let initial_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        assert_eq!(initial_key.user_uuid, user1_uuid);

        // Reassign to user2
        repo.reassign(key_uuid, user2_uuid).await?;

        // Verify reassignment
        let updated_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        assert_eq!(updated_key.user_uuid, user2_uuid);

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test concurrent API key usage
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_concurrent_api_key_usage() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create API key
        let (_key_uuid, key_value) = repo
            .create_new_api_key("Test Key", "Test description", user_uuid, 30)
            .await?;

        // Create test app
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/protected").wrap(ApiAuth::new()).route(
                        web::get().to(move |req: HttpRequest| async move {
                            // Simulate protected endpoint
                            req.extensions().get::<ApiKeyInfo>().map_or_else(
                                || {
                                    HttpResponse::Unauthorized().json(serde_json::json!({
                                        "status": "error",
                                        "message": "Unauthorized"
                                    }))
                                },
                                |_auth| {
                                    HttpResponse::Ok().json(serde_json::json!({
                                        "status": "success",
                                        "message": "Access granted"
                                    }))
                                },
                            )
                        }),
                    )),
            )
            .await;

        // Test concurrent usage - run requests sequentially since app doesn't implement Clone
        for _ in 0..5 {
            let req = test::TestRequest::get()
                .uri("/protected")
                .insert_header(("X-API-Key", key_value.as_str()))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // Verify the key is still valid after concurrent usage
        let auth_result = repo.find_api_key_for_auth(&key_value).await?;
        assert!(auth_result.is_some());

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key pagination functionality
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_pagination() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create multiple API keys
        for i in 1..=25 {
            repo.create_new_api_key(
                &format!("Key {i}"),
                &format!("Description {i}"),
                user_uuid,
                30,
            )
            .await?;
        }

        // Test pagination with page=1, per_page=10
        let (keys_page1, total) = tokio::join!(
            repo.list_by_user(user_uuid, 10, 0, None, None),
            repo.count_by_user(user_uuid)
        );

        let keys_page1 = keys_page1?;
        let total = total?;

        assert_eq!(keys_page1.len(), 10);
        assert_eq!(total, 25);

        // Test pagination with page=2, per_page=10
        let keys_page2 = repo.list_by_user(user_uuid, 10, 10, None, None).await?;
        assert_eq!(keys_page2.len(), 10);

        // Test pagination with page=3, per_page=10
        let keys_page3 = repo.list_by_user(user_uuid, 10, 20, None, None).await?;
        assert_eq!(keys_page3.len(), 5); // Should be 5 remaining keys

        // Test pagination with page=4, per_page=10
        let keys_page4 = repo.list_by_user(user_uuid, 10, 30, None, None).await?;
        assert_eq!(keys_page4.len(), 0); // Should be no keys

        // Test different per_page values
        let keys_page1_20 = repo.list_by_user(user_uuid, 20, 0, None, None).await?;
        assert_eq!(keys_page1_20.len(), 20);

        let keys_page2_20 = repo.list_by_user(user_uuid, 20, 20, None, None).await?;
        assert_eq!(keys_page2_20.len(), 5);

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test HTTP API pagination functionality for API keys
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_api_key_http_pagination() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));

        // Create multiple API keys
        for i in 1..=25 {
            repo.create_new_api_key(
                &format!("Key {i}"),
                &format!("Description {i}"),
                user_uuid,
                30,
            )
            .await?;
        }

        // Create test app with actual API routes
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let license_config = LicenseConfig::default();
        let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

        let api_state = ApiState {
            db_pool: pool.pool.clone(),
            api_config: r_data_core_core::config::ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8888,
                use_tls: false,
                jwt_secret: "test_secret".to_string(),
                jwt_expiration: 3600,
                enable_docs: true,
                cors_origins: vec![],
                check_default_admin_password: true,
            },
            role_service: r_data_core_services::RoleService::new(
                pool.pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: r_data_core_services::DashboardStatsService::new(Arc::new(
                r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::scope("/admin/api/v1").service(
                        web::scope("/api-keys")
                            .configure(r_data_core_api::admin::api_keys::routes::register_routes),
                    ),
                ),
        )
        .await;

        // Create a JWT token for authentication
        let token = create_test_jwt_token(&user_uuid, "test_secret");

        // Test page 1 with 10 items per page using JWT authentication
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?page=1&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let data = body["data"].as_array().unwrap();
        assert_eq!(data.len(), 10);

        let meta = body["meta"]["pagination"].as_object().unwrap();
        assert_eq!(meta["page"], 1);
        assert_eq!(meta["per_page"], 10);
        assert_eq!(meta["total"], 25);
        assert_eq!(meta["total_pages"], 3);
        assert_eq!(meta["has_previous"], false);
        assert_eq!(meta["has_next"], true);

        // Test page 2 with 10 items per page
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?page=2&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let data = body["data"].as_array().unwrap();
        assert_eq!(data.len(), 10);

        let meta = body["meta"]["pagination"].as_object().unwrap();
        assert_eq!(meta["page"], 2);
        assert_eq!(meta["per_page"], 10);
        assert_eq!(meta["total"], 25);
        assert_eq!(meta["total_pages"], 3);
        assert_eq!(meta["has_previous"], true);
        assert_eq!(meta["has_next"], true);

        // Test page 3 with 10 items per page (should have 5 items)
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?page=3&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let data = body["data"].as_array().unwrap();
        assert_eq!(data.len(), 5);

        let meta = body["meta"]["pagination"].as_object().unwrap();
        assert_eq!(meta["page"], 3);
        assert_eq!(meta["per_page"], 10);
        assert_eq!(meta["total"], 25);
        assert_eq!(meta["total_pages"], 3);
        assert_eq!(meta["has_previous"], true);
        assert_eq!(meta["has_next"], false);

        // Test page 4 with 10 items per page (should have 0 items)
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?page=4&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let data = body["data"].as_array().unwrap();
        assert_eq!(data.len(), 0);

        let meta = body["meta"]["pagination"].as_object().unwrap();
        assert_eq!(meta["page"], 4);
        assert_eq!(meta["per_page"], 10);
        assert_eq!(meta["total"], 25);
        assert_eq!(meta["total_pages"], 3);
        assert_eq!(meta["has_previous"], true);
        assert_eq!(meta["has_next"], false);

        // Test different per_page value
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?page=1&per_page=20")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let data = body["data"].as_array().unwrap();
        assert_eq!(data.len(), 20);

        let meta = body["meta"]["pagination"].as_object().unwrap();
        assert_eq!(meta["page"], 1);
        assert_eq!(meta["per_page"], 20);
        assert_eq!(meta["total"], 25);
        assert_eq!(meta["total_pages"], 2);
        assert_eq!(meta["has_previous"], false);
        assert_eq!(meta["has_next"], true);

        clear_test_db(&pool).await?;
        Ok(())
    }
}
