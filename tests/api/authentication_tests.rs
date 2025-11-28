use actix_web::{
    http::{header, StatusCode},
    test, web, App, HttpMessage, HttpRequest, HttpResponse,
};
use r_data_core_api::jwt::AuthUserClaims;
use r_data_core_api::{
    middleware::{ApiAuth, ApiKeyInfo},
    ApiState,
};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, ApiKeyRepositoryTrait,
};
use r_data_core_services::{AdminUserService, ApiKeyService, EntityDefinitionService};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::utils;
    use serial_test::serial;

    /// Test API key authentication with valid key
    #[tokio::test]
    #[serial]
    async fn test_api_key_authentication_valid() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create API key
        let (key_uuid, key_value) = api_key_repo
            .create_new_api_key("TestKey", "Test Description", user_uuid, 30)
            .await?;

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: utils::make_workflow_service(&pool),
            queue: utils::test_queue_client_async().await,
        };

        // Create test app with API key authentication middleware
        let app = test::init_service(
            App::new()
                .wrap(r_data_core_api::middleware::create_error_handlers())
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/test")
                        .wrap(r_data_core_api::middleware::ApiAuth)
                        .to(|req: HttpRequest| async move {
                            // Check if API key info was added to request extensions
                            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No API key found"
                                }))
                            }
                        }),
                ),
        )
        .await;

        // Test with valid API key
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["api_key_uuid"], key_uuid.to_string());

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key authentication with invalid key
    #[tokio::test]
    #[serial]
    async fn test_api_key_authentication_invalid() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: crate::common::utils::make_workflow_service(&pool),
            queue: crate::common::utils::test_queue_client_async().await,
        };

        // Create test app with API key authentication middleware
        let app =
            test::init_service(
                App::new()
                    .wrap(r_data_core_api::middleware::create_error_handlers())
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/test").wrap(ApiAuth).to(
                        |req: HttpRequest| async move {
                            // Check if API key info was added to request extensions
                            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No API key found"
                                }))
                            }
                        },
                    )),
            )
            .await;

        // Test with invalid API key
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", "invalid_key"))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(result.is_err(), "Expected an error for invalid API key");

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test API key authentication with missing header
    #[tokio::test]
    #[serial]
    async fn test_api_key_authentication_missing_header() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: crate::common::utils::make_workflow_service(&pool),
            queue: crate::common::utils::test_queue_client_async().await,
        };

        // Create test app with API key authentication middleware
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/test")
                        .wrap(r_data_core_api::middleware::ApiAuth)
                        .to(|req: HttpRequest| async move {
                            // Check if API key info was added to request extensions
                            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No API key found"
                                }))
                            }
                        }),
                ),
        )
        .await;

        // Test with no API key header
        let req = test::TestRequest::get().uri("/test").to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(
            result.is_err(),
            "Expected an error for missing API key header"
        );

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test combined authentication with API key
    #[tokio::test]
    #[serial]
    async fn test_combined_auth_api_key() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create the API key
        let (_key_uuid, key_value) = api_key_repo
            .create_new_api_key("TestKey", "Test Description", user_uuid, 30)
            .await?;

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: crate::common::utils::make_workflow_service(&pool),
            queue: crate::common::utils::test_queue_client_async().await,
        };

        // Create test app with combined authentication middleware
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/test")
                        .wrap(r_data_core_api::middleware::CombinedAuth)
                        .to(|req: HttpRequest| async move {
                            // Check if authentication info was added to request extensions
                            if let Some(claims) = req.extensions().get::<AuthUserClaims>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "auth_method": "jwt",
                                    "user_uuid": claims.sub
                                }))
                            } else if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>()
                            {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "auth_method": "api_key",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No authentication found"
                                }))
                            }
                        }),
                ),
        )
        .await;

        // Test with API key
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["auth_method"], "api_key");

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test combined authentication with no credentials
    #[tokio::test]
    #[serial]
    async fn test_combined_auth_no_credentials() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create API key
        let (_key_uuid, _key_value) = api_key_repo
            .create_new_api_key(
                "TestKey",
                "Test Description",
                utils::create_test_admin_user(&pool).await?,
                30,
            )
            .await?;

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
                r_data_core_services::WorkflowRepositoryAdapter::new(
                    r_data_core_persistence::WorkflowRepository::new(pool.clone()),
                ),
            )),
            queue: crate::common::utils::test_queue_client_async().await,
        };

        // Create test app with combined authentication middleware
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/test")
                        .wrap(r_data_core_api::middleware::CombinedAuth)
                        .to(|req: HttpRequest| async move {
                            // Check if authentication info was added to request extensions
                            if let Some(claims) = req.extensions().get::<AuthUserClaims>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "auth_method": "jwt",
                                    "user_uuid": claims.sub
                                }))
                            } else if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>()
                            {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "auth_method": "api_key",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "Error",
                                    "message": "No authentication found"
                                }))
                            }
                        }),
                ),
        )
        .await;

        // Test with no credentials
        let req = test::TestRequest::get().uri("/test").to_request();

        let resp = test::call_service(&app, req).await;
        // Middleware should allow request to proceed, handler returns unauthorized
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["status"], "Error");

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test expired API key authentication
    #[tokio::test]
    #[serial]
    async fn test_expired_api_key_authentication() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create API key with no expiration
        let (key_uuid, key_value) = api_key_repo
            .create_new_api_key("TestKey", "Test Description", user_uuid, 0) // No expiration
            .await?;

        // Manually expire the key by setting expires_at to the past
        sqlx::query!(
            "UPDATE api_keys SET expires_at = NOW() - INTERVAL '1 day' WHERE uuid = $1",
            key_uuid
        )
        .execute(&pool)
        .await?;

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
                r_data_core_services::WorkflowRepositoryAdapter::new(
                    r_data_core_persistence::WorkflowRepository::new(pool.clone()),
                ),
            )),
            queue: crate::common::utils::test_queue_client_async().await,
        };

        // Create test app with API key authentication middleware
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(
                    web::resource("/test")
                        .wrap(r_data_core_api::middleware::ApiAuth)
                        .to(|req: HttpRequest| async move {
                            // Check if API key info was added to request extensions
                            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No API key found"
                                }))
                            }
                        }),
                ),
        )
        .await;

        // Test with expired API key
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(result.is_err(), "Expected an error for expired API key");

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test revoked API key authentication
    #[tokio::test]
    #[serial]
    async fn test_revoked_api_key_authentication() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create API key
        let (key_uuid, key_value) = api_key_repo
            .create_new_api_key("TestKey", "Test Description", user_uuid, 30)
            .await?;

        // Revoke the API key
        api_key_repo.revoke(key_uuid).await?;

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

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
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: utils::make_workflow_service(&pool),
            queue: utils::test_queue_client_async().await,
        };

        // Create test app with API key authentication middleware
        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                        api_state,
                    )))
                    .service(web::resource("/test").wrap(ApiAuth).to(
                        |req: HttpRequest| async move {
                            // Check if API key info was added to request extensions
                            if let Some(api_key_info) = req.extensions().get::<ApiKeyInfo>() {
                                HttpResponse::Ok().json(serde_json::json!({
                                    "status": "success",
                                    "api_key_uuid": api_key_info.uuid,
                                    "user_uuid": api_key_info.user_uuid
                                }))
                            } else {
                                HttpResponse::Unauthorized().json(serde_json::json!({
                                    "status": "error",
                                    "message": "No API key found"
                                }))
                            }
                        },
                    )),
            )
            .await;

        // Test with revoked API key
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-API-Key", key_value))
            .to_request();

        let result = test::try_call_service(&app, req).await;
        assert!(result.is_err(), "Expected an error for revoked API key");

        utils::clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test JWT authentication middleware with valid token
    #[tokio::test]
    #[serial]
    async fn test_jwt_middleware_valid_token() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.clone(),
        ));

        // Create cache config
        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));
        let jwt_secret = "test_secret";

        // Generate a valid JWT token
        let user = admin_user_repo.find_by_uuid(&user_uuid).await?.unwrap();
        let api_config = r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: jwt_secret.to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        };
        let token = r_data_core_api::jwt::generate_access_token(&user, &api_config, None)
            .expect("Failed to generate JWT token");

        let api_state = ApiState {
            db_pool: pool.clone(),
            api_config: api_config.clone(),
            permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
                pool.clone(),
                cache_manager.clone(),
                Some(0),
            ),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: utils::make_workflow_service(&pool),
            queue: utils::test_queue_client_async().await,
        };

        // Create test app with JWT authentication middleware
        let app = test::init_service(
            App::new()
                .wrap(r_data_core_api::middleware::create_error_handlers())
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(web::resource("/test").to(
                    |auth: r_data_core_api::auth::auth_enum::RequiredAuth| async move {
                        // RequiredAuth extractor ensures JWT is valid
                        HttpResponse::Ok().json(serde_json::json!({
                            "status": "success",
                            "auth_method": "jwt",
                            "user_uuid": auth.0.sub
                        }))
                    },
                )),
        )
        .await;

        // Test with valid JWT token
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = test::read_body(resp).await;
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["auth_method"], "jwt");
        assert_eq!(response["user_uuid"], user_uuid.to_string());

        utils::clear_test_db(&pool).await?;
        Ok(())
    }
}
