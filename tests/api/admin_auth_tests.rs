#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Basic test setup for Admin Auth routes
// More comprehensive tests to be implemented

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, web, App};
    use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
    use r_data_core_core::cache::CacheManager;
    use r_data_core_core::config::{CacheConfig, LicenseConfig};
    use r_data_core_persistence::{
        AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, CreateAdminUserParams,
        DashboardStatsRepository, EntityDefinitionRepository,
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
    use uuid::Uuid;

    #[tokio::test]
    async fn test_admin_user_last_login_update() -> r_data_core_core::error::Result<()> {
        // Setup test database
        use r_data_core_test_support::{clear_test_db, setup_test_db};

        let pool = setup_test_db().await;

        // Clear any existing data
        clear_test_db(&pool)
            .await
            .expect("Failed to clear test database");

        // Create a repository to work with admin users directly
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

        // Generate a username and email that won't conflict
        let unique_id = Uuid::now_v7().to_string()[0..8].to_string();
        let username = format!("test_user_{unique_id}");
        let email = format!("test{unique_id}@example.com");
        let test_password = "Test123!";

        // Create the admin user directly in the database
        let params = CreateAdminUserParams {
            username: &username,
            email: &email,
            password: test_password,
            first_name: "Test",
            last_name: "User",
            role: None,
            is_active: true,
            creator_uuid: Uuid::now_v7(),
        };
        let user_uuid = repo.create_admin_user(&params).await?;

        // Verify the user has no last_login timestamp initially
        let initial_user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert!(
            initial_user.last_login.is_none(),
            "New user should have no last_login timestamp"
        );

        // Simulate a login by updating the last_login timestamp
        repo.update_last_login(&user_uuid).await?;

        // Fetch the user again to check if last_login was updated
        let updated_user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert!(
            updated_user.last_login.is_some(),
            "User's last_login should be updated after login"
        );

        // For a more comprehensive test, we would also verify JWT creation and validation
        // This would require calling the actual login endpoint, which would be part of an API test

        Ok(())
    }

    #[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
    async fn setup_test_app_with_config(
        check_default_password: bool,
    ) -> r_data_core_core::error::Result<(
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
                check_default_admin_password: check_default_password,
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
        };

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
                .configure(configure_app),
        )
        .await;

        Ok((app, pool))
    }

    #[tokio::test]
    #[serial]
    async fn test_login_response_includes_default_password_check_when_enabled(
    ) -> r_data_core_core::error::Result<()> {
        // Create admin user with default password hash
        const DEFAULT_PASSWORD_HASH: &str =
            "$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4";

        let (app, pool) = setup_test_app_with_config(true).await?;

        let user_uuid = Uuid::now_v7();
        sqlx::query!(
            r#"
            INSERT INTO admin_users (
                uuid, path, username, email, password_hash, first_name, last_name,
                is_active, created_at, updated_at, published, version, created_by
            ) VALUES (
                $1, '/users', 'admin', 'admin@example.com', $2, 'System', 'Administrator',
                true, NOW(), NOW(), true, 1, $1
            )
            "#,
            user_uuid,
            DEFAULT_PASSWORD_HASH
        )
        .execute(&pool.pool)
        .await?;

        // Create a test user to login with
        let test_user_uuid = create_test_admin_user(&pool).await?;
        let test_user = AdminUserRepository::new(Arc::new(pool.pool.clone()))
            .find_by_uuid(&test_user_uuid)
            .await?
            .unwrap();

        // Login with test user
        let login_req = test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": test_user.username,
                "password": "adminadmin"
            }))
            .to_request();

        let resp = test::call_service(&app, login_req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["data"]["using_default_password"] != serde_json::Value::Null,
            "Response should include using_default_password field"
        );
        assert_eq!(
            body["data"]["using_default_password"],
            serde_json::json!(true),
            "Should indicate default password is in use"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_login_response_when_password_changed() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_test_app_with_config(true).await?;

        // Create admin user with different password hash (not the default)
        let admin_user_uuid = Uuid::now_v7();
        let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
        let params = CreateAdminUserParams {
            username: "admin",
            email: "admin@example.com",
            password: "NewPassword123!",
            first_name: "System",
            last_name: "Administrator",
            role: None,
            is_active: true,
            creator_uuid: admin_user_uuid,
        };
        repo.create_admin_user(&params).await?;

        // Create a test user to login with
        let test_user_uuid = create_test_admin_user(&pool).await?;
        let test_user = repo.find_by_uuid(&test_user_uuid).await?.unwrap();

        // Login with test user
        let login_req = test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": test_user.username,
                "password": "adminadmin"
            }))
            .to_request();

        let resp = test::call_service(&app, login_req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body["data"]["using_default_password"] != serde_json::Value::Null,
            "Response should include using_default_password field"
        );
        assert_eq!(
            body["data"]["using_default_password"],
            serde_json::json!(false),
            "Should indicate default password is not in use"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_login_response_when_check_disabled() -> r_data_core_core::error::Result<()> {
        let (app, pool) = setup_test_app_with_config(false).await?;

        // Create a test user to login with
        let test_user_uuid = create_test_admin_user(&pool).await?;
        let test_user = AdminUserRepository::new(Arc::new(pool.pool.clone()))
            .find_by_uuid(&test_user_uuid)
            .await?
            .unwrap();

        // Login with test user
        let login_req = test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": test_user.username,
                "password": "adminadmin"
            }))
            .to_request();

        let resp = test::call_service(&app, login_req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["data"]["using_default_password"],
            serde_json::json!(false),
            "Response should have false using_default_password when check is disabled"
        );

        clear_test_db(&pool).await?;
        Ok(())
    }
}
