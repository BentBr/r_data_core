#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{
    http::{header, StatusCode},
    test, web, App,
};
use r_data_core_api::jwt::AuthUserClaims;
use r_data_core_api::ApiState;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_core::error::Result;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
use r_data_core_services::{AdminUserService, ApiKeyService, EntityDefinitionService};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn create_test_jwt_token(user_uuid: &Uuid, secret: &str) -> String {
    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(1);

    // SuperAdmin gets all permissions
    let permissions = [
        "workflows:read",
        "workflows:create",
        "workflows:update",
        "workflows:delete",
        "workflows:execute",
        "entities:read",
        "entities:create",
        "entities:update",
        "entities:delete",
        "entity_definitions:read",
        "entity_definitions:create",
        "entity_definitions:update",
        "entity_definitions:delete",
        "api_keys:read",
        "api_keys:create",
        "api_keys:update",
        "api_keys:delete",
        "roles:read",
        "roles:create",
        "roles:update",
        "roles:delete",
        "system:read",
        "system:create",
        "system:update",
        "system:delete",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();

    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        name: "test_user".to_string(),
        email: "test@example.com".to_string(),
        permissions,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        exp: exp.unix_timestamp() as usize,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        iat: now.unix_timestamp() as usize,
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
    use r_data_core_core::config::LicenseConfig;
    use r_data_core_services::LicenseService;
    use r_data_core_test_support::{
        clear_test_db, create_test_admin_user, setup_test_db, test_queue_client_async,
    };
    use serial_test::serial;

    /// Test HTTP API pagination functionality for entity definitions
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_entity_definition_http_pagination() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;

        // Create multiple entity definitions
        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        for i in 1..=25 {
            let entity_def = r_data_core_core::entity_definition::definition::EntityDefinition {
                uuid: Uuid::now_v7(),
                entity_type: format!("test_entity_{i}"),
                display_name: format!("Test Entity {i}"),
                description: Some(format!("Description for test entity {i}")),
                group_name: Some("test_group".to_string()),
                allow_children: false,
                icon: Some("mdi-test".to_string()),
                fields: vec![],
                schema: r_data_core_core::entity_definition::schema::Schema::default(),
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
                created_by: user_uuid,
                updated_by: Some(user_uuid),
                published: true,
                version: 1,
            };

            EntityDefinitionRepositoryTrait::create(entity_def_repo.as_ref(), &entity_def).await?;
        }

        // Create test app with actual API routes
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let dashboard_stats_repository =
            r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
        let dashboard_stats_service =
            r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
            workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
                r_data_core_services::WorkflowRepositoryAdapter::new(
                    r_data_core_persistence::WorkflowRepository::new(pool.pool.clone()),
                ),
            )),
            dashboard_stats_service,
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(web::scope("/admin/api/v1").service(
                    web::scope("/entity-definitions").configure(
                        r_data_core_api::admin::entity_definitions::routes::register_routes,
                    ),
                )),
        )
        .await;

        // Create a JWT token for authentication
        let token = create_test_jwt_token(&user_uuid, "test_secret");

        // Test page 1 with 10 items per page
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/entity-definitions?page=1&per_page=10")
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
            .uri("/admin/api/v1/entity-definitions?page=2&per_page=10")
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
            .uri("/admin/api/v1/entity-definitions?page=3&per_page=10")
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
            .uri("/admin/api/v1/entity-definitions?page=4&per_page=10")
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
            .uri("/admin/api/v1/entity-definitions?page=1&per_page=20")
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

    /// Test that entity definition UUID is correctly set after creation
    ///
    /// This test verifies that when creating an entity definition via the HTTP API,
    /// the UUID is properly set in the database and can be retrieved correctly.
    /// Previously, the cached version had a nil UUID instead of the actual UUID.
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_entity_definition_uuid_after_creation() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;

        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 3600, // Enable caching with TTL
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let dashboard_stats_repository =
            r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
        let dashboard_stats_service =
            r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
            entity_definition_service: EntityDefinitionService::new(
                entity_def_repo,
                cache_manager.clone(),
            ),
            dynamic_entity_service: None,
            workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
                r_data_core_services::WorkflowRepositoryAdapter::new(
                    r_data_core_persistence::WorkflowRepository::new(pool.pool.clone()),
                ),
            )),
            dashboard_stats_service,
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(web::scope("/admin/api/v1").service(
                    web::scope("/entity-definitions").configure(
                        r_data_core_api::admin::entity_definitions::routes::register_routes,
                    ),
                )),
        )
        .await;

        // Create a JWT token for authentication
        let token = create_test_jwt_token(&user_uuid, "test_secret");

        // Create an entity definition via HTTP API
        let entity_def_payload = serde_json::json!({
            "entity_type": "customer",
            "display_name": "Customer",
            "description": "regular customer with default fields",
            "group_name": "",
            "allow_children": false,
            "icon": "user",
            "fields": [],
            "published": true
        });

        let req = test::TestRequest::post()
            .uri("/admin/api/v1/entity-definitions")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(&entity_def_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        // Extract the UUID from the creation response
        let created_uuid_str = body["data"]["uuid"]
            .as_str()
            .expect("UUID should be present in response");
        let created_uuid = Uuid::parse_str(created_uuid_str).expect("UUID should be valid");

        // Verify UUID is not nil
        assert_ne!(created_uuid, Uuid::nil(), "Created UUID should not be nil");

        // Retrieve the entity definition by UUID via HTTP API
        let req = test::TestRequest::get()
            .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        // Verify the UUID in the retrieved response matches the created UUID
        let retrieved_uuid_str = body["data"]["uuid"]
            .as_str()
            .expect("UUID should be present in response");
        let retrieved_uuid = Uuid::parse_str(retrieved_uuid_str).expect("UUID should be valid");

        assert_eq!(
            retrieved_uuid, created_uuid,
            "Retrieved UUID should match created UUID"
        );
        assert_ne!(
            retrieved_uuid,
            Uuid::nil(),
            "Retrieved UUID should not be nil"
        );

        // Verify other fields are correct
        assert_eq!(body["data"]["entity_type"], "customer");
        assert_eq!(body["data"]["display_name"], "Customer");

        // Test that retrieving by UUID works even after cache is populated
        // (this tests that the cache has the correct UUID)
        let req = test::TestRequest::get()
            .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let cached_uuid_str = body["data"]["uuid"]
            .as_str()
            .expect("UUID should be present in cached response");
        let cached_uuid = Uuid::parse_str(cached_uuid_str).expect("UUID should be valid");

        assert_eq!(
            cached_uuid, created_uuid,
            "Cached UUID should match created UUID"
        );
        assert_ne!(cached_uuid, Uuid::nil(), "Cached UUID should not be nil");

        clear_test_db(&pool).await?;
        Ok(())
    }

    /// Test that Json field type is correctly preserved through create and retrieve
    ///
    /// This test verifies that when creating an entity definition with Json field type,
    /// the field type is correctly preserved and returned as "Json" (not converted to "Object").
    ///
    /// # Errors
    /// Returns an error if the test setup or API call fails
    #[tokio::test]
    #[serial]
    async fn test_json_field_type_preserved_through_api() -> Result<()> {
        let pool = setup_test_db().await;
        let user_uuid = create_test_admin_user(&pool).await?;

        let entity_def_repo = Arc::new(r_data_core_persistence::EntityDefinitionRepository::new(
            pool.pool.clone(),
        ));

        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 3600,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let dashboard_stats_repository =
            r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
        let dashboard_stats_service =
            r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
            entity_definition_service: EntityDefinitionService::new(
                entity_def_repo,
                cache_manager.clone(),
            ),
            dynamic_entity_service: None,
            workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
                r_data_core_services::WorkflowRepositoryAdapter::new(
                    r_data_core_persistence::WorkflowRepository::new(pool.pool.clone()),
                ),
            )),
            dashboard_stats_service,
            queue: test_queue_client_async().await,
            license_service,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                    api_state,
                )))
                .service(web::scope("/admin/api/v1").service(
                    web::scope("/entity-definitions").configure(
                        r_data_core_api::admin::entity_definitions::routes::register_routes,
                    ),
                )),
        )
        .await;

        let token = create_test_jwt_token(&user_uuid, "test_secret");

        // Create an entity definition with Json, Object, and Array field types
        // This mimics the StatisticSubmission entity definition use case
        let entity_def_payload = serde_json::json!({
            "entity_type": "statistic_submission",
            "display_name": "Statistic Submission",
            "description": "Statistics from instances",
            "group_name": "system",
            "allow_children": false,
            "icon": "chart-bar",
            "fields": [
                {
                    "name": "cors_origins",
                    "display_name": "CORS Origins",
                    "field_type": "Json",
                    "description": "Array of allowed CORS origins",
                    "required": false,
                    "indexed": false,
                    "filterable": false
                },
                {
                    "name": "entities_per_definition",
                    "display_name": "Entities Per Definition",
                    "field_type": "Json",
                    "description": "Array of entity type counts",
                    "required": false,
                    "indexed": false,
                    "filterable": false
                },
                {
                    "name": "entity_definitions",
                    "display_name": "Entity Definitions",
                    "field_type": "Json",
                    "description": "Object with count and names",
                    "required": false,
                    "indexed": false,
                    "filterable": false
                },
                {
                    "name": "settings",
                    "display_name": "Settings",
                    "field_type": "Object",
                    "description": "Object field (must be JSON object)",
                    "required": false,
                    "indexed": false,
                    "filterable": false
                },
                {
                    "name": "tags",
                    "display_name": "Tags",
                    "field_type": "Array",
                    "description": "Array field",
                    "required": false,
                    "indexed": false,
                    "filterable": false
                }
            ],
            "published": true
        });

        // Create the entity definition
        let req = test::TestRequest::post()
            .uri("/admin/api/v1/entity-definitions")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(&entity_def_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        let created_uuid_str = body["data"]["uuid"]
            .as_str()
            .expect("UUID should be present in response");
        let created_uuid = Uuid::parse_str(created_uuid_str).expect("UUID should be valid");

        // Retrieve the entity definition and verify field types are preserved
        let req = test::TestRequest::get()
            .uri(&format!("/admin/api/v1/entity-definitions/{created_uuid}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Success");

        // Verify field types are preserved correctly
        let fields = body["data"]["fields"]
            .as_array()
            .expect("fields should be array");
        assert_eq!(fields.len(), 5);

        // Find and verify each field type
        let cors_field = fields
            .iter()
            .find(|f| f["name"] == "cors_origins")
            .expect("cors_origins field should exist");
        assert_eq!(
            cors_field["field_type"], "Json",
            "cors_origins should be Json, not Object"
        );

        let entities_per_def_field = fields
            .iter()
            .find(|f| f["name"] == "entities_per_definition")
            .expect("entities_per_definition field should exist");
        assert_eq!(
            entities_per_def_field["field_type"], "Json",
            "entities_per_definition should be Json, not Object"
        );

        let entity_defs_field = fields
            .iter()
            .find(|f| f["name"] == "entity_definitions")
            .expect("entity_definitions field should exist");
        assert_eq!(
            entity_defs_field["field_type"], "Json",
            "entity_definitions should be Json, not Object"
        );

        let settings_field = fields
            .iter()
            .find(|f| f["name"] == "settings")
            .expect("settings field should exist");
        assert_eq!(
            settings_field["field_type"], "Object",
            "settings should be Object"
        );

        let tags_field = fields
            .iter()
            .find(|f| f["name"] == "tags")
            .expect("tags field should exist");
        assert_eq!(tags_field["field_type"], "Array", "tags should be Array");

        // Test listing endpoint also returns correct field types
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/entity-definitions?page=1&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        let definitions = body["data"].as_array().expect("data should be array");
        let stat_def = definitions
            .iter()
            .find(|d| d["entity_type"] == "statistic_submission")
            .expect("statistic_submission should be in list");

        let list_fields = stat_def["fields"]
            .as_array()
            .expect("fields should be array");
        let list_cors_field = list_fields
            .iter()
            .find(|f| f["name"] == "cors_origins")
            .expect("cors_origins field should exist in list");
        assert_eq!(
            list_cors_field["field_type"], "Json",
            "cors_origins should be Json in list response too"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }
}
