use actix_web::{
    http::{header, StatusCode},
    test, web, App,
};
use r_data_core::{
    api::ApiState,
    config::CacheConfig,
    entity::admin_user::{AdminUserRepository, ApiKeyRepository},
};
use r_data_core_services::{AdminUserService, ApiKeyService, EntityDefinitionService};
use r_data_core_core::error::Result;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_api::jwt::AuthUserClaims;
use r_data_core_core::cache::CacheManager;
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
        "permission_schemes:read".to_string(),
        "permission_schemes:create".to_string(),
        "permission_schemes:update".to_string(),
        "permission_schemes:delete".to_string(),
        "system:read".to_string(),
        "system:create".to_string(),
        "system:update".to_string(),
        "system:delete".to_string(),
    ];

    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        name: "test_user".to_string(),
        email: "test@example.com".to_string(),
        role: "SuperAdmin".to_string(),
        permissions,
        exp: exp.unix_timestamp() as usize,
        iat: now.unix_timestamp() as usize,
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
    use crate::common::utils;
    use serial_test::serial;

    /// Test HTTP API pagination functionality for entity definitions
    #[tokio::test]
    #[serial]
    async fn test_entity_definition_http_pagination() -> Result<()> {
        let pool = utils::setup_test_db().await;
        let user_uuid = utils::create_test_admin_user(&pool).await?;

        // Create multiple entity definitions
        let entity_def_repo = Arc::new(
            r_data_core_persistence::EntityDefinitionRepository::new(
                pool.clone(),
            ),
        );

        for i in 1..=25 {
            let entity_def = r_data_core_core::entity_definition::definition::EntityDefinition {
                uuid: Uuid::now_v7(),
                entity_type: format!("test_entity_{}", i),
                display_name: format!("Test Entity {}", i),
                description: Some(format!("Description for test entity {}", i)),
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

            entity_def_repo.create(&entity_def).await?;
        }

        // Create test app with actual API routes
        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.clone()));

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };

        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ApiState {
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
                    entity_definition_service: EntityDefinitionService::new_without_cache(
                        entity_def_repo,
                    ),
                    dynamic_entity_service: None,
                    workflow_service: r_data_core::services::WorkflowService::new(Arc::new(
                        r_data_core::services::WorkflowRepositoryAdapter::new(
                            r_data_core::workflow::data::repository::WorkflowRepository::new(
                                pool.clone(),
                            ),
                        ),
                    )),
                    queue: crate::common::utils::test_queue_client_async().await,
                }))
                .service(web::scope("/admin/api/v1").service(
                    web::scope("/entity-definitions").configure(
                        r_data_core::api::admin::entity_definitions::routes::register_routes,
                    ),
                )),
        )
        .await;

        // Create a JWT token for authentication
        let token = create_test_jwt_token(&user_uuid, "test_secret");

        // Test page 1 with 10 items per page
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/entity-definitions?page=1&per_page=10")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
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
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
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
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
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
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
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
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
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

        utils::clear_test_db(&pool).await?;
        Ok(())
    }
}
