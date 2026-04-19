//! Bootstrap module for initialising application components
//!
//! This module provides functions to set up the application's core infrastructure
//! including database connections, caching, and services.

#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

use r_data_core_api::ApiState;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::AppConfig;
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, DashboardStatsRepository, DynamicEntityRepository,
    EmailTemplateRepository, EntityDefinitionRepository, PasswordResetRepository,
    SystemLogRepository, WorkflowRepository,
};
use r_data_core_services::adapters::{
    AdminUserRepositoryAdapter, ApiKeyRepositoryAdapter, DynamicEntityRepositoryAdapter,
    EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, DynamicEntityService,
    EntityDefinitionService, LicenseService, MailService, PasswordResetService, RoleService,
    SystemLogService, WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

/// Initialise the environment logger with the given log level
pub fn init_logger(log_level: &str) {
    let env = env_logger::Env::new().default_filter_or(log_level);
    env_logger::Builder::from_env(env)
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .format_module_path(true)
        .format_target(true)
        .init();
}

/// Create a database connection pool
///
/// # Errors
/// Returns an error if the database connection fails
pub async fn create_db_pool(config: &AppConfig) -> r_data_core_core::error::Result<PgPool> {
    info!("Connecting to database...");
    PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!(
                "Failed to create database connection pool: {e}"
            ))
        })
}

/// Initialise the cache manager with Redis backend
///
/// # Errors
/// Returns an error if Redis URL is empty or if Redis connection fails
pub async fn create_cache_manager(
    config: &AppConfig,
) -> r_data_core_core::error::Result<Arc<CacheManager>> {
    let redis_url = &config.queue.redis_url;

    if redis_url.is_empty() {
        return Err(r_data_core_core::error::Error::Config(
            "Redis URL is required but was empty".to_string(),
        ));
    }

    let manager = CacheManager::new(config.cache.clone())
        .with_redis(redis_url)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to initialize Redis cache: {e}"))
        })?;

    info!("Cache manager initialized with Redis");
    Ok(Arc::new(manager))
}

/// Verify licence on startup
///
/// This function verifies the licence and logs the result without blocking startup.
pub async fn verify_license_on_startup(config: &AppConfig, cache_manager: Arc<CacheManager>) {
    let license_service = LicenseService::new(config.license.clone(), cache_manager);
    license_service.verify_license_on_startup("core").await;
}

/// Initialise the Redis queue client for workflows
///
/// # Errors
/// Returns an error if the Redis queue connection fails
pub async fn create_queue_client(
    config: &AppConfig,
) -> r_data_core_core::error::Result<Arc<ApalisRedisQueue>> {
    info!("Initializing Redis queue client...");
    let queue = ApalisRedisQueue::from_parts(
        &config.queue.redis_url,
        &config.queue.fetch_key,
        &config.queue.process_key,
        "queue:email",
    )
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Config(format!(
            "Failed to initialize Redis queue client: {e}"
        ))
    })?;

    Ok(Arc::new(queue))
}

/// Build the complete API state with all services initialised
///
/// # Errors
/// Returns an error if queue initialisation fails
///
/// # Panics
/// Does not panic under normal conditions
pub async fn build_api_state(
    config: &AppConfig,
    pool: PgPool,
    cache_manager: Arc<CacheManager>,
) -> r_data_core_core::error::Result<ApiState> {
    // Create repositories
    let pool_arc = Arc::new(pool.clone());
    let api_key_repository = ApiKeyRepository::new(pool_arc.clone());
    let admin_user_repository = AdminUserRepository::new(pool_arc);
    let entity_definition_repository = EntityDefinitionRepository::new(pool.clone());
    let dynamic_entity_repository =
        DynamicEntityRepository::with_cache(pool.clone(), cache_manager.clone());

    // Create services with adapters
    // Initialise system log service (created early so it can be injected into other services)
    let system_log_repository = SystemLogRepository::new(pool.clone());
    let system_log_service = Arc::new(SystemLogService::new(Arc::new(system_log_repository)));

    let api_key_adapter = ApiKeyRepositoryAdapter::new(api_key_repository);
    let api_key_service = ApiKeyService::with_cache(
        Arc::new(api_key_adapter),
        cache_manager.clone(),
        config.cache.api_key_ttl,
    )
    .with_system_log(system_log_service.clone());

    let admin_user_adapter = AdminUserRepositoryAdapter::new(admin_user_repository);
    let admin_user_service = AdminUserService::new(Arc::new(admin_user_adapter))
        .with_system_log(system_log_service.clone());

    let entity_definition_adapter =
        EntityDefinitionRepositoryAdapter::new(entity_definition_repository);
    let entity_definition_service =
        EntityDefinitionService::new(Arc::new(entity_definition_adapter), cache_manager.clone())
            .with_system_log(system_log_service.clone());

    let dynamic_entity_adapter =
        DynamicEntityRepositoryAdapter::from_repository(dynamic_entity_repository);
    let dynamic_entity_service = DynamicEntityService::new(
        Arc::new(dynamic_entity_adapter),
        Arc::new(entity_definition_service.clone()),
    );

    let workflow_repo = WorkflowRepository::new(pool.clone());
    let workflow_adapter = WorkflowRepositoryAdapter::new(workflow_repo);
    let workflow_mail_service =
        config
            .mail
            .workflow
            .as_ref()
            .and_then(|smtp| match MailService::new(smtp) {
                Ok(svc) => Some(Arc::new(svc)),
                Err(e) => {
                    log::warn!("Failed to initialize workflow mail service: {e}");
                    None
                }
            });
    let workflow_service = WorkflowService::new(Arc::new(workflow_adapter))
        .with_jwt_config(
            Some(config.api.jwt_secret.clone()),
            config.api.jwt_expiration,
        )
        .with_mail_service(workflow_mail_service)
        .with_system_log(system_log_service.clone());

    let role_service = RoleService::new(
        pool.clone(),
        cache_manager.clone(),
        Some(config.cache.entity_definition_ttl),
    )
    .with_system_log(system_log_service.clone());

    let dashboard_stats_repository = DashboardStatsRepository::new(pool.clone());
    let dashboard_stats_service = DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    // Initialise licence service
    let license_service = LicenseService::new(config.license.clone(), cache_manager.clone());

    // Initialise queue client
    let queue_client = create_queue_client(config).await?;

    // Initialise password reset service if system mail is configured
    let password_reset_service = config.mail.system.as_ref().map_or_else(
        || {
            log::debug!("System mail not configured; password reset disabled");
            None
        },
        |smtp_config| match MailService::new(smtp_config) {
            Ok(mail_service) => {
                let pool_arc2 = Arc::new(pool.clone());
                let password_reset_repo = PasswordResetRepository::new(pool.clone());
                let user_repo_for_reset =
                    r_data_core_persistence::AdminUserRepository::new(pool_arc2);
                let template_repo = EmailTemplateRepository::new(pool.clone());
                let frontend_base_url = config.frontend_base_url.clone().unwrap_or_default();
                Some(PasswordResetService::new(
                    Arc::new(password_reset_repo),
                    Arc::new(user_repo_for_reset),
                    Arc::new(template_repo),
                    queue_client.clone(),
                    Arc::new(mail_service),
                    config.password_reset_throttle_seconds,
                    frontend_base_url,
                ))
            }
            Err(e) => {
                log::warn!("Failed to initialize mail service for password reset: {e}");
                None
            }
        },
    );

    Ok(ApiState {
        db_pool: pool,
        api_config: config.api.clone(),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(Arc::new(dynamic_entity_service)),
        workflow_service,
        role_service,
        dashboard_stats_service,
        license_service: Arc::new(license_service),
        queue: queue_client,
        password_reset_service,
        system_log_service: Some(system_log_service),
    })
}
