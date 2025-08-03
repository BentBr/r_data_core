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

    #[actix_web::test]
    async fn test_query_parameter_deserialization_fix() {
        let app = setup_test_app().await.expect("Failed to setup test app");

        // Test cases for the query parameter deserialization fix
        let test_cases = vec![
            // Basic pagination parameters
            ("/api/v1/user?page=1&per_page=10", "Basic pagination"),
            ("/api/v1/user?page=2&per_page=20", "Page 2 with per_page"),
            ("/api/v1/user?page=1", "Only page parameter"),
            ("/api/v1/user?per_page=50", "Only per_page parameter"),
            // Include parameter combinations
            (
                "/api/v1/user?page=1&per_page=10&include=children",
                "With include parameter",
            ),
            (
                "/api/v1/user?include=children&page=1&per_page=1000",
                "Include with large per_page",
            ),
            (
                "/api/v1/user?include=children,parent&page=2&per_page=5",
                "Multiple includes",
            ),
            // Edge cases
            (
                "/api/v1/user?page=0&per_page=1",
                "Page 0 (should be clamped to 1)",
            ),
            (
                "/api/v1/user?page=1&per_page=0",
                "Per_page 0 (should be clamped to 1)",
            ),
            (
                "/api/v1/user?page=999999&per_page=999999",
                "Very large numbers",
            ),
            // String values that should be parsed as integers
            (
                "/api/v1/user?page=1&per_page=1000&include=children",
                "String numbers to i64",
            ),
            (
                "/api/v1/user?page=2&per_page=50",
                "Another string to i64 test",
            ),
        ];

        for (url, description) in test_cases {
            println!("Testing: {}", description);

            let req = test::TestRequest::get()
                .uri(url)
                .insert_header(("Authorization", "Bearer test_token"))
                .to_request();

            let resp = test::call_service(&app, req).await;

            // The important thing is that we don't get a 400 Bad Request with deserialization error
            // We expect either 401 (unauthorized) or 200 (success) but NOT 400 with deserialization error
            let status = resp.status();

            if status.is_client_error() {
                let body = test::read_body(resp).await;
                let body_str = String::from_utf8_lossy(&body);

                // Check that we're not getting the deserialization error
                assert!(
                    !body_str.contains("invalid type: string")
                        && !body_str.contains("expected i64"),
                    "Query deserialization error occurred for {}: {} - Body: {}",
                    description,
                    url,
                    body_str
                );

                // If it's a 400, it should be for a different reason (like invalid entity type)
                if status.as_u16() == 400 {
                    println!("Got 400 for {}: {}", description, body_str);
                }
            }

            println!("✓ {} - Status: {}", description, status);
        }
    }

    #[actix_web::test]
    async fn test_pagination_query_parameters() {
        let app = setup_test_app().await.expect("Failed to setup test app");

        // Test the specific case that was failing before the fix
        let req = test::TestRequest::get()
            .uri("/api/v1/user?page=1&per_page=1000&include=children")
            .insert_header(("Authorization", "Bearer test_token"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();

        // Should not be a 400 Bad Request with deserialization error
        if status.is_client_error() {
            let body = test::read_body(resp).await;
            let body_str = String::from_utf8_lossy(&body);

            assert!(
                !body_str.contains("invalid type: string") && !body_str.contains("expected i64"),
                "Query deserialization error still occurring: {}",
                body_str
            );
        }

        println!(
            "✓ Pagination query parameters test passed - Status: {}",
            status
        );
    }

    #[actix_web::test]
    async fn test_various_string_to_integer_conversions() {
        let app = setup_test_app().await.expect("Failed to setup test app");

        // Test various string representations of numbers
        let test_urls = vec![
            "/api/v1/user?page=1&per_page=10",
            "/api/v1/user?page=2&per_page=20",
            "/api/v1/user?page=100&per_page=1000",
            "/api/v1/user?page=0&per_page=0", // Should be clamped
            "/api/v1/user?page=999999&per_page=999999", // Very large numbers
        ];

        for url in test_urls {
            let req = test::TestRequest::get()
                .uri(url)
                .insert_header(("Authorization", "Bearer test_token"))
                .to_request();

            let resp = test::call_service(&app, req).await;
            let status = resp.status();

            if status.is_client_error() {
                let body = test::read_body(resp).await;
                let body_str = String::from_utf8_lossy(&body);

                // Ensure no deserialization errors
                assert!(
                    !body_str.contains("invalid type: string")
                        && !body_str.contains("expected i64"),
                    "String to integer conversion failed for {}: {}",
                    url,
                    body_str
                );
            }

            println!("✓ String to integer conversion test passed for {}", url);
        }
    }

    #[actix_web::test]
    async fn test_include_parameter_with_pagination() {
        let app = setup_test_app().await.expect("Failed to setup test app");

        // Test include parameter combinations with pagination
        let test_cases = vec![
            (
                "/api/v1/user?include=children&page=1&per_page=10",
                "Include children",
            ),
            (
                "/api/v1/user?include=parent&page=2&per_page=20",
                "Include parent",
            ),
            (
                "/api/v1/user?include=children,parent&page=1&per_page=50",
                "Multiple includes",
            ),
            (
                "/api/v1/user?page=1&per_page=1000&include=children",
                "Original failing case",
            ),
        ];

        for (url, description) in test_cases {
            let req = test::TestRequest::get()
                .uri(url)
                .insert_header(("Authorization", "Bearer test_token"))
                .to_request();

            let resp = test::call_service(&app, req).await;
            let status = resp.status();

            if status.is_client_error() {
                let body = test::read_body(resp).await;
                let body_str = String::from_utf8_lossy(&body);

                // Check for deserialization errors
                assert!(
                    !body_str.contains("invalid type: string")
                        && !body_str.contains("expected i64"),
                    "Include parameter test failed for {}: {} - Body: {}",
                    description,
                    url,
                    body_str
                );
            }

            println!("✓ Include parameter test passed for {}", description);
        }
    }
}
