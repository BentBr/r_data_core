#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

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
            pool.clone(),
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

        let dashboard_stats_repository =
            r_data_core_persistence::DashboardStatsRepository::new(pool.clone());
        let dashboard_stats_service =
            r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
            role_service: r_data_core_services::RoleService::new(
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
            dashboard_stats_service,
            queue: test_queue_client_async().await,
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
}
