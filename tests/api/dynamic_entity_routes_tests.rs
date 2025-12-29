#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{test, web, App};
use r_data_core_api::{configure_app, ApiState};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
};
use std::sync::Arc;

// Import common test utilities
use r_data_core_test_support::{
    clear_test_db, create_test_api_key, create_test_entity, create_test_entity_definition,
    make_workflow_service, setup_test_db, test_queue_client_async,
};

#[cfg(test)]
#[allow(clippy::module_inception)]
mod dynamic_entity_api_tests {
    use super::*;

    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn setup_test_app() -> Result<(
        impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
        r_data_core_test_support::TestDatabase,
    )> {
        // Setup database
        let pool = setup_test_db().await;
        clear_test_db(&pool.pool).await?;

        // Create required services
        let cache_config = CacheConfig {
            entity_definition_ttl: 0, // No expiration
            api_key_ttl: 600,         // 10 minutes for tests
            enabled: true,
            ttl: 3600, // 1-hour default
            max_size: 10000,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));

        // Create user entity definition
        let _ = create_test_entity_definition(&pool, "user").await?;

        // Create test users with paths to exercise folder browsing
        for i in 1..=3 {
            let uuid = create_test_entity(
                &pool,
                "user",
                &format!("Root User {i}"),
                &format!("root{i}@example.com"),
            )
            .await?;
            // Update path in entities_registry to root
            sqlx::query("UPDATE entities_registry SET path = '/', entity_key = $2 WHERE uuid = $1")
                .bind(uuid)
                .bind(format!("root-{i}"))
                .execute(&pool.pool)
                .await?;
        }

        // Add users under /team and /team/dev
        let u1 = create_test_entity(&pool, "user", "Alice", "alice@example.com").await?;
        sqlx::query(
            "UPDATE entities_registry SET path = '/team', entity_key = 'alice' WHERE uuid = $1",
        )
        .bind(u1)
        .execute(&pool.pool)
        .await?;

        let u2 = create_test_entity(&pool, "user", "Bob", "bob@example.com").await?;
        sqlx::query(
            "UPDATE entities_registry SET path = '/team/dev', entity_key = 'bob' WHERE uuid = $1",
        )
        .bind(u2)
        .execute(&pool.pool)
        .await?;

        // Create an API key
        let api_key = "test_api_key_12345";
        create_test_api_key(&pool, api_key.to_string()).await?;

        // Create services
        let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
        let api_key_service = ApiKeyService::new(api_key_repository);

        let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
        let admin_user_service = AdminUserService::new(admin_user_repository);

        let entity_definition_repository =
            Arc::new(EntityDefinitionRepository::new(pool.pool.clone()));
        let entity_definition_service =
            EntityDefinitionService::new_without_cache(entity_definition_repository);

        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.pool.clone()));
        let dynamic_entity_service = Arc::new(DynamicEntityService::new(
            dynamic_entity_repository,
            Arc::new(entity_definition_service.clone()),
        ));

        let dashboard_stats_repository =
            r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
        let dashboard_stats_service =
            r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

        // Create app state
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
            cache_manager,
            api_key_service,
            admin_user_service,
            entity_definition_service,
            dynamic_entity_service: Some(dynamic_entity_service),
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service,
            queue: test_queue_client_async().await,
        };

        let app_data = web::Data::new(r_data_core_api::ApiStateWrapper::new(api_state));

        // Build test app
        let app = test::init_service(
            App::new()
                .app_data(app_data.clone())
                .configure(configure_app),
        )
        .await;

        Ok((app, pool))
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_query_parameter_deserialization_fix() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

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
            println!("Testing: {description}");

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
                    "Query deserialization error occurred for {description}: {url} - Body: {body_str}"
                );

                // If it's a 400, it should be for a different reason (like invalid entity type)
                if status.as_u16() == 400 {
                    println!("Got 400 for {description}: {body_str}");
                }
            }

            println!("✓ {description} - Status: {status}");
        }
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_pagination_query_parameters() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

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
                "Query deserialization error still occurring: {body_str}"
            );
        }

        println!("✓ Pagination query parameters test passed - Status: {status}");
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_various_string_to_integer_conversions() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

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
                    "String to integer conversion failed for {url}: {body_str}"
                );
            }

            println!("✓ String to integer conversion test passed for {url}");
        }
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_browse_by_path_endpoint() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

        // Browse root
        let req = test::TestRequest::get()
            .uri("/api/v1/entities/by-path?path=/&limit=50")
            .insert_header(("X-API-Key", "test_api_key_12345"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let s = String::from_utf8_lossy(&body);
        assert!(s.contains("\"status\":\"Success\""));
        // Expect to see folder 'team' in the result and some files
        assert!(s.contains("\"kind\":\"folder\""));

        // Browse /team should show folder 'dev' and maybe a file
        let req2 = test::TestRequest::get()
            .uri("/api/v1/entities/by-path?path=/team&limit=50")
            .insert_header(("X-API-Key", "test_api_key_12345"))
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert!(resp2.status().is_success());
        let body2 = test::read_body(resp2).await;
        let s2 = String::from_utf8_lossy(&body2);
        assert!(s2.contains("\"name\":\"dev\""));
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_unique_key_per_path_conflict() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

        // Create one entity under /projects with key "alpha"
        let req1 = test::TestRequest::post()
            .uri("/api/v1/user")
            .insert_header(("X-API-Key", "test_api_key_12345"))
            .set_json(serde_json::json!({
                "name": "Proj Owner",
                "email": "owner@example.com",
                "path": "/projects",
                "entity_key": "alpha"
            }))
            .to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert!(resp1.status().is_success());

        // Try to create another with same key in same path -> expect 409
        let req2 = test::TestRequest::post()
            .uri("/api/v1/user")
            .insert_header(("X-API-Key", "test_api_key_12345"))
            .set_json(serde_json::json!({
                "name": "Dup",
                "email": "dup@example.com",
                "path": "/projects",
                "entity_key": "alpha"
            }))
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status().as_u16(), 409);
    }

    #[actix_web::test]
    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn test_include_parameter_with_pagination() {
        let (app, _db) = setup_test_app().await.expect("Failed to setup test app");

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
                    "Include parameter test failed for {description}: {url} - Body: {body_str}"
                );
            }

            println!("✓ Include parameter test passed for {description}");
        }
    }
}
