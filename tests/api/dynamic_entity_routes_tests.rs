use actix_web::{test, web, App};
use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::api::{configure_app, ApiState};
use r_data_core::cache::CacheManager;
use r_data_core::config::CacheConfig;
use r_data_core::entity::admin_user::repository::{AdminUserRepository, ApiKeyRepository};
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::error::Result;
use r_data_core::services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
};
use std::sync::Arc;

// Import common test utilities
#[path = "../common/mod.rs"]
mod common;

#[cfg(test)]
mod dynamic_entity_api_tests {
    use super::*;

    async fn setup_test_app() -> Result<
        impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
    > {
        // Setup database
        let pool = common::utils::setup_test_db().await;
        common::utils::clear_test_db(&pool).await?;

        // Create required services
        let cache_config = CacheConfig {
            enabled: true,
            ttl: 300,
            max_size: 10000,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));

        // Create user entity definition
        let user_def_uuid = common::utils::create_test_entity_definition(&pool, "user").await?;

        // Create test users
        for i in 1..=5 {
            common::utils::create_test_entity(
                &pool,
                "user",
                &format!("Test User {}", i),
                &format!("user{}@example.com", i),
            )
            .await?;
        }

        // Create an API key
        let api_key = "test_api_key_12345";
        common::utils::create_test_api_key(&pool, api_key.to_string()).await?;

        // Create services
        let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.clone())));
        let api_key_service = ApiKeyService::new(api_key_repository);

        let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
        let admin_user_service = AdminUserService::new(admin_user_repository);

        let entity_definition_repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let entity_definition_service = EntityDefinitionService::new(entity_definition_repository);

        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let dynamic_entity_service = Arc::new(DynamicEntityService::new(
            dynamic_entity_repository,
            Arc::new(entity_definition_service.clone()),
        ));

        // Create app state
        let app_state = web::Data::new(ApiState {
            db_pool: pool.clone(),
            jwt_secret: "test_secret".to_string(),
            cache_manager,
            api_key_service,
            admin_user_service,
            entity_definition_service,
            dynamic_entity_service: Some(dynamic_entity_service),
        });

        // Build test app
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .configure(configure_app),
        )
        .await;

        Ok(app)
    }

    // The tests will be written here
}
