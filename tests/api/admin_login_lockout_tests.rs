#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

// Integration tests for admin login security hardening:
//   1. Account lockout after 5 consecutive failed attempts
//   2. Successful login resets the failed-attempts counter
//
// Rate-limit (429) path is NOT tested here: the actix test harness does not
// wire a real `peer_addr` to the request, so `req.peer_addr()` returns `None`
// and all attempts share the key `login_rl:unknown`.  The in-process
// `CacheManager` (non-Redis) used by `setup_test_app_with_config` IS enabled,
// which means these tests could inadvertently collide when run in parallel
// against the same cache key.  Because the cache is scoped to a single
// `Arc<CacheManager>` instance that is created fresh per test, tests are
// isolated, BUT since `peer_addr` is always "unknown" the 429 would only
// fire after ≥ 10 failures from successive test runs sharing that instance.
// Given each test creates its own `CacheManager`, the 429 path is not
// reliably reachable without injecting a fake IP.  It is therefore skipped.

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, web, App};
    use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
    use r_data_core_core::cache::CacheManager;
    use r_data_core_core::config::{CacheConfig, LicenseConfig};
    use r_data_core_persistence::{
        AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, DashboardStatsRepository,
        EntityDefinitionRepository,
    };
    use r_data_core_services::{
        AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService,
        LicenseService, RoleService,
    };
    use r_data_core_test_support::{
        clear_test_db, create_test_admin_user, make_workflow_service, setup_test_db,
        test_queue_client_async,
    };
    use serial_test::serial;
    use std::sync::Arc;

    // ---------------------------------------------------------------------------
    // Shared test-app factory (mirrors admin_auth_tests.rs setup)
    // ---------------------------------------------------------------------------

    async fn setup_app() -> r_data_core_core::error::Result<(
        impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
        r_data_core_test_support::TestDatabase,
    )> {
        let pool = setup_test_db().await;
        clear_test_db(&pool).await?;

        let cache_config = CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        };
        let cache_manager = Arc::new(CacheManager::new(cache_config));

        let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
        let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let entity_def_repo = Arc::new(EntityDefinitionRepository::new(pool.pool.clone()));

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
                check_default_admin_password: false,
            },
            role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(0)),
            cache_manager: cache_manager.clone(),
            api_key_service: ApiKeyService::from_repository(api_key_repo),
            admin_user_service: AdminUserService::from_repository(admin_user_repo),
            entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
            dynamic_entity_service: None,
            workflow_service: make_workflow_service(&pool),
            dashboard_stats_service: DashboardStatsService::new(Arc::new(
                DashboardStatsRepository::new(pool.pool.clone()),
            )),
            queue: test_queue_client_async().await,
            license_service,
            password_reset_service: None,
            system_log_service: None,
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
                .configure(configure_app),
        )
        .await;

        Ok((app, pool))
    }

    // ---------------------------------------------------------------------------
    // Helper: attempt login and return the HTTP status code
    // ---------------------------------------------------------------------------

    async fn attempt_login(
        app: &impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
        username: &str,
        password: &str,
    ) -> StatusCode {
        let req = test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": username,
                "password": password
            }))
            .to_request();
        test::call_service(app, req).await.status()
    }

    // ---------------------------------------------------------------------------
    // Test 1: account lockout after 5 consecutive bad-password attempts
    // ---------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn test_account_locked_after_five_failures() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_app().await?;

        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        let username = user.username.clone();

        // Five wrong-password attempts → each must return 401
        for attempt in 1_u8..=5 {
            let status = attempt_login(&app, &username, "wrong_password").await;
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "attempt {attempt} should return 401"
            );
        }

        // Sixth attempt with the CORRECT password → account is now Locked → 403
        let status = attempt_login(&app, &username, "adminadmin").await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "correct-password login after lockout should be 403"
        );

        // Verify the DB row reflects the locked state
        let locked_user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert!(
            locked_user.failed_login_attempts >= 5,
            "failed_login_attempts should be >= 5, got {}",
            locked_user.failed_login_attempts
        );
        assert_eq!(
            locked_user.status,
            r_data_core_core::admin_user::UserStatus::Locked,
            "status should be Locked"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Test 2: fewer than 5 failures then a successful login resets the counter
    // ---------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn test_successful_login_resets_failure_counter() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_app().await?;

        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        let username = user.username.clone();

        // Three bad attempts (below the lockout threshold of 5)
        for attempt in 1_u8..=3 {
            let status = attempt_login(&app, &username, "wrong_password").await;
            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "attempt {attempt} should return 401"
            );
        }

        // Verify counter has been incremented
        let user_after_failures = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert_eq!(
            user_after_failures.failed_login_attempts, 3,
            "counter should be 3 after three failures"
        );

        // Successful login must return 200
        let status = attempt_login(&app, &username, "adminadmin").await;
        assert_eq!(
            status,
            StatusCode::OK,
            "correct-password login should be 200"
        );

        // Counter must be reset to 0 in the DB
        let user_after_success = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert_eq!(
            user_after_success.failed_login_attempts, 0,
            "failed_login_attempts should be reset to 0 after a successful login"
        );
        assert_eq!(
            user_after_success.status,
            r_data_core_core::admin_user::UserStatus::Active,
            "status should remain Active after successful login"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Test 3: locked account also rejected when status was pre-set to Locked
    //         (ensures can_login() check fires independently of how it was locked)
    // ---------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn test_pre_locked_account_returns_403() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_app().await?;

        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let user = repo.find_by_uuid(&user_uuid).await?.unwrap();

        // Manually lock the account in the DB (simulates an admin action or a lockout
        // that was persisted in a previous session).
        sqlx::query(
            "UPDATE admin_users SET status = 'Locked', failed_login_attempts = 5 WHERE uuid = $1",
        )
        .bind(user_uuid)
        .execute(&pool.pool)
        .await
        .expect("failed to pre-lock account");

        // Login with the correct password must be rejected with 403
        let status = attempt_login(&app, &user.username, "adminadmin").await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "pre-locked account should return 403 even with the correct password"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Test 4: inactive account (is_active = false) returns 403 (can_login = false)
    // ---------------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn test_inactive_account_returns_403() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_app().await?;

        let user_uuid = create_test_admin_user(&pool).await?;
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let user = repo.find_by_uuid(&user_uuid).await?.unwrap();

        // Deactivate the account
        sqlx::query("UPDATE admin_users SET is_active = false WHERE uuid = $1")
            .bind(user_uuid)
            .execute(&pool.pool)
            .await
            .expect("failed to deactivate account");

        let status = attempt_login(&app, &user.username, "adminadmin").await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "inactive account should return 403"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }
}
