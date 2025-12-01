#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use log::{debug, error, info};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

// Todo: These modules will be implemented later
// mod notification;

use r_data_core_api::ApiResponse;
use r_data_core_api::{ApiState, ApiStateWrapper};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::load_app_config;
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
use r_data_core_services::adapters::ApiKeyRepositoryAdapter;
use r_data_core_services::adapters::{
    AdminUserRepositoryAdapter, DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
    PermissionSchemeService,
};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

// 404 handler function
async fn default_404_handler() -> impl actix_web::Responder {
    ApiResponse::<()>::not_found("Resource not found")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables and configure the application
    let config = match load_app_config() {
        Ok(cfg) => {
            debug!("Loaded conf: {:?}", cfg);
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            panic!("Failed to load configuration: {}", e);
        }
    };

    // Initialize logger
    let env = env_logger::Env::new().default_filter_or(&config.log.level);
    env_logger::Builder::from_env(env)
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .format_module_path(true)
        .format_target(true)
        .init();

    info!("Starting R Data Core server...");
    info!("Environment: {}", config.environment);
    info!("Log level: {}", config.log.level);
    info!("API docs enabled: {}", config.api.enable_docs);

    // Create a database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations using SQLx instead of custom Rust migrations
    // Note: Run SQLx migrations with `cargo sqlx migrate run` before starting the application
    info!("Using SQLx migrations (run with 'cargo sqlx migrate run')");

    // Initialize cache manager
    let redis_url = std::env::var("REDIS_URL").ok();

    let cache_manager = match redis_url {
        Some(url) if !url.is_empty() => {
            match CacheManager::new(config.cache.clone())
                .with_redis(&url)
                .await
            {
                Ok(manager) => {
                    info!("Cache manager initialized with Redis");
                    Arc::new(manager)
                }
                Err(e) => {
                    error!(
                        "Failed to initialize Redis cache: {}, falling back to in-memory only",
                        e
                    );
                    Arc::new(CacheManager::new(config.cache.clone()))
                }
            }
        }
        _ => {
            info!("Redis URL not provided, using in-memory cache only");
            Arc::new(CacheManager::new(config.cache.clone()))
        }
    };

    // Initialize repositories and services
    let pool_arc = Arc::new(pool.clone());
    let api_key_repository = ApiKeyRepository::new(pool_arc.clone());
    let admin_user_repository = AdminUserRepository::new(pool_arc.clone());
    let entity_definition_repository = EntityDefinitionRepository::new(pool.clone());
    let dynamic_entity_repository =
        DynamicEntityRepository::with_cache(pool.clone(), cache_manager.clone());

    // Initialize services with adapters
    let api_key_adapter = ApiKeyRepositoryAdapter::new(api_key_repository);
    let api_key_service = ApiKeyService::with_cache(
        Arc::new(api_key_adapter),
        cache_manager.clone(),
        config.cache.api_key_ttl,
    );

    // Use adapter for AdminUserRepository
    let admin_user_adapter = AdminUserRepositoryAdapter::new(admin_user_repository);
    let admin_user_service = AdminUserService::new(Arc::new(admin_user_adapter));

    // Use the adapter for EntityDefinitionRepository
    let entity_definition_adapter =
        EntityDefinitionRepositoryAdapter::new(entity_definition_repository);
    let entity_definition_service =
        EntityDefinitionService::new(Arc::new(entity_definition_adapter), cache_manager.clone());

    // Initialize dynamic entity service
    let dynamic_entity_adapter =
        DynamicEntityRepositoryAdapter::from_repository(dynamic_entity_repository);
    let dynamic_entity_service = DynamicEntityService::new(
        Arc::new(dynamic_entity_adapter),
        Arc::new(entity_definition_service.clone()),
    );

    // Shared application state
    let workflow_repo = WorkflowRepository::new(pool.clone());
    let workflow_adapter = WorkflowRepositoryAdapter::new(workflow_repo);
    let workflow_service = WorkflowService::new(Arc::new(workflow_adapter));

    // Initialize permission scheme service
    let permission_scheme_service = PermissionSchemeService::new(
        pool.clone(),
        cache_manager.clone(),
        Some(config.cache.entity_definition_ttl), // Use entity_definition_ttl for scheme caching
    );
    // Initialize mandatory queue client (fail fast if invalid)
    let queue_client = Arc::new(
        ApalisRedisQueue::from_parts(
            &config.queue.redis_url,
            &config.queue.fetch_key,
            &config.queue.process_key,
        )
        .await
        .expect("Failed to initialize Redis queue client"),
    );

    let api_state = ApiState {
        db_pool: pool,
        api_config: config.api.clone(),
        cache_manager: cache_manager.clone(),
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(Arc::new(dynamic_entity_service)),
        workflow_service,
        permission_scheme_service,
        queue: queue_client.clone(),
    };

    let app_state = web::Data::new(ApiStateWrapper::new(api_state));

    let bind_address = format!("{}:{}", config.api.host, config.api.port);
    info!("Starting HTTP server at http://{}", bind_address);

    // Start HTTP server
    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(vec!["content-disposition"])
            .max_age(3600);

        let api_config = r_data_core_api::ApiConfiguration {
            enable_auth: false,  // Todo
            enable_admin: true,  // Todo
            enable_public: true, // Todo
            enable_docs: config.api.enable_docs,
        };

        App::new()
            .app_data(app_state.clone())
            .wrap(r_data_core_api::middleware::create_error_handlers())
            .wrap(Logger::new("%a %{User-Agent}i %r %s %D"))
            .wrap(cors)
            .configure(move |cfg| r_data_core_api::configure_app_with_options(cfg, api_config))
            .default_service(web::route().to(default_404_handler))
    })
    .bind(bind_address)?
    .run()
    .await
}
