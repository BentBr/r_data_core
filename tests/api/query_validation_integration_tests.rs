#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{http::StatusCode, test, web, App};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, DashboardStatsRepository, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService, RoleService,
};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, setup_test_db, test_queue_client_async,
};
use serial_test::serial;
use std::sync::Arc;

use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};

async fn setup_test_app() -> Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    r_data_core_test_support::TestDatabase,
    String, // auth_token
)> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;

    let _user_uuid = create_test_admin_user(&pool.pool).await?;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = WorkflowService::new(Arc::new(wf_adapter));

    let dashboard_stats_repository = DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service = DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
        },
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600)),
        cache_manager: cache_manager.clone(),
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
        dashboard_stats_service,
        queue: test_queue_client_async().await,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    // Get auth token
    let username: String = sqlx::query_scalar(
        "SELECT username FROM admin_users WHERE super_admin = true ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&pool.pool)
    .await
    .expect("Test admin user should exist");

    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": "adminadmin"
        }))
        .to_request();

    let resp = test::call_service(&app, login_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    Ok((app, pool, token))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test users endpoint with valid `sort_by` and `sort_order`
    #[tokio::test]
    #[serial]
    async fn test_users_list_valid_sorting() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        // Test with valid sort field
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?sort_by=username&sort_order=asc")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Valid sort parameters should return 200"
        );
    }

    /// Test users endpoint with invalid `sort_by` field
    #[tokio::test]
    #[serial]
    async fn test_users_list_invalid_sort_field() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        // Test with non-existing field
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?sort_by=non_existing_field_xyz")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Invalid sort field should return 400"
        );

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Error");
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("Invalid sort field"),
            "Error message should mention invalid sort field"
        );
    }

    /// Test users endpoint with invalid `sort_order`
    #[tokio::test]
    #[serial]
    async fn test_users_list_invalid_sort_order() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        // Test with invalid sort order
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?sort_by=username&sort_order=invalid")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Invalid sort order should return 400"
        );

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "Error");
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("Invalid sort_order"),
            "Error message should mention invalid sort_order"
        );
    }

    /// Test users endpoint with SQL injection attempt in `sort_by`
    #[tokio::test]
    #[serial]
    async fn test_users_list_sql_injection_sort_by() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        // Test SQL injection attempts
        let sql_injection_attempts = vec![
            "'; DROP TABLE users; --",
            "username'; DELETE FROM users",
            "username' OR '1'='1",
        ];

        for attempt in sql_injection_attempts {
            // URL encode the attempt manually for common cases
            let encoded = attempt
                .replace(' ', "%20")
                .replace('\'', "%27")
                .replace(';', "%3B");
            let uri = format!("/admin/api/v1/users?sort_by={encoded}");
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "SQL injection attempt '{attempt}' should return 400"
            );
        }
    }

    /// Test users endpoint with invalid pagination - negative page
    #[tokio::test]
    #[serial]
    async fn test_users_list_invalid_page() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?page=-1")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Invalid page should return 400"
        );
    }

    /// Test users endpoint with invalid pagination - `per_page` too large
    #[tokio::test]
    #[serial]
    async fn test_users_list_invalid_per_page() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?per_page=1000")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Invalid per_page should return 400"
        );
    }

    /// Test users endpoint with `per_page` = -1 (unlimited)
    #[tokio::test]
    #[serial]
    async fn test_users_list_unlimited() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?per_page=-1")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "per_page = -1 should be allowed for admin endpoints"
        );
    }

    /// Test api-keys endpoint with valid sorting
    #[tokio::test]
    #[serial]
    async fn test_api_keys_list_valid_sorting() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?sort_by=name&sort_order=asc")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Test api-keys endpoint with invalid sort field
    #[tokio::test]
    #[serial]
    async fn test_api_keys_list_invalid_sort_field() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/api-keys?sort_by=invalid_field")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// Test roles endpoint with valid sorting
    #[tokio::test]
    #[serial]
    async fn test_roles_list_valid_sorting() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/roles?sort_by=name&sort_order=desc")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Test roles endpoint with invalid sort field
    #[tokio::test]
    #[serial]
    async fn test_roles_list_invalid_sort_field() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/roles?sort_by=non_existing")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// Test workflows endpoint with valid sorting
    #[tokio::test]
    #[serial]
    async fn test_workflows_list_valid_sorting() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/workflows?sort_by=name&sort_order=asc")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Test workflows endpoint with invalid sort field
    #[tokio::test]
    #[serial]
    async fn test_workflows_list_invalid_sort_field() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let req = test::TestRequest::get()
            .uri("/admin/api/v1/workflows?sort_by=invalid_field")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// Test case insensitivity for `sort_order`
    #[tokio::test]
    #[serial]
    async fn test_sort_order_case_insensitive() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let cases = vec!["ASC", "asc", "Asc", "DESC", "desc", "DeSc"];

        for case in cases {
            let uri = format!("/admin/api/v1/users?sort_by=username&sort_order={case}");
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "sort_order '{case}' should be accepted (case insensitive)"
            );
        }
    }

    /// Test edge case: empty `sort_by` parameter
    #[tokio::test]
    #[serial]
    async fn test_empty_sort_by() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        // Empty sort_by should be handled gracefully (no sorting or default sorting)
        let req = test::TestRequest::get()
            .uri("/admin/api/v1/users?sort_by=")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should either return 400 (invalid) or 200 (default sorting)
        // The current implementation should return 400 for empty field
        assert!(
            resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::OK,
            "Empty sort_by should be handled"
        );
    }

    /// Test edge case: special characters in `sort_by` (should be rejected)
    #[tokio::test]
    #[serial]
    async fn test_special_characters_sort_by() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let special_chars = vec!["field@name", "field#name", "field$name", "field%name"];

        for field in special_chars {
            // URL encode special characters
            let encoded = field
                .replace('@', "%40")
                .replace('#', "%23")
                .replace('$', "%24")
                .replace('%', "%25");
            let uri = format!("/admin/api/v1/users?sort_by={encoded}");
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "Special characters in sort_by '{field}' should be rejected"
            );
        }
    }

    /// Test valid pagination combinations
    #[tokio::test]
    #[serial]
    async fn test_valid_pagination_combinations() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let valid_combinations = vec![
            "?page=1&per_page=10",
            "?page=2&per_page=20",
            "?limit=50&offset=0",
            "?limit=25&offset=25",
            "?per_page=100",
            "?page=1",
        ];

        for params in valid_combinations {
            let uri = format!("/admin/api/v1/users{params}");
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "Valid pagination '{params}' should return 200"
            );
        }
    }

    /// Test invalid pagination combinations
    #[tokio::test]
    #[serial]
    async fn test_invalid_pagination_combinations() {
        let (app, _pool, token) = setup_test_app().await.unwrap();

        let invalid_combinations = vec![
            "?page=0",
            "?page=-1",
            "?per_page=0",
            "?per_page=-2",
            "?per_page=101",
            "?limit=0",
            "?limit=-1",
            "?limit=101",
            "?offset=-1",
        ];

        for params in invalid_combinations {
            let uri = format!("/admin/api/v1/users{params}");
            let req = test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Authorization", format!("Bearer {token}")))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status(),
                StatusCode::BAD_REQUEST,
                "Invalid pagination '{params}' should return 400"
            );
        }
    }
}
