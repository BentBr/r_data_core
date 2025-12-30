#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use anyhow::Context;
use log::{debug, info};

use r_data_core::bootstrap::{build_api_state, create_cache_manager, create_db_pool, init_logger};
use r_data_core_api::{ApiResponse, ApiStateWrapper};
use r_data_core_core::config::load_app_config;

/// 404 handler function
async fn default_404_handler() -> impl actix_web::Responder {
    ApiResponse::<()>::not_found("Resource not found")
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = load_app_config()
        .map_err(|e| anyhow::anyhow!("Failed to load application configuration: {e}"))?;
    debug!("Loaded conf: {config:?}");

    // Initialize logger
    init_logger(&config.log.level);

    info!("Starting R Data Core server...");
    info!("Environment: {}", config.environment);
    info!("Log level: {}", config.log.level);
    info!("API docs enabled: {}", config.api.enable_docs);

    // Create database pool
    let pool = create_db_pool(&config)
        .await
        .context("Failed to create database connection pool")?;

    info!("Using SQLx migrations (run with 'cargo sqlx migrate run')");

    // Initialize cache manager
    let cache_manager = create_cache_manager(&config)
        .await
        .context("Failed to initialize cache manager with Redis")?;

    // Build API state with all services
    let api_state = build_api_state(&config, pool, cache_manager)
        .await
        .context("Failed to initialize API state")?;

    let app_state = web::Data::new(ApiStateWrapper::new(api_state));

    let bind_address = format!("{}:{}", config.api.host, config.api.port);
    let bind_address_clone = bind_address.clone();
    info!("Starting HTTP server at http://{bind_address}");

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(vec!["content-disposition"])
            .max_age(3600);

        let api_config = r_data_core_api::ApiConfiguration {
            enable_auth: false,
            enable_admin: true,
            enable_public: true,
            enable_docs: config.api.enable_docs,
        };

        App::new()
            .app_data(app_state.clone())
            .wrap(r_data_core_api::middleware::create_error_handlers())
            .wrap(Logger::new("%a %{User-Agent}i %r %s %D"))
            .wrap(cors)
            .configure(move |cfg| r_data_core_api::configure_app_with_options(cfg, &api_config))
            .default_service(web::route().to(default_404_handler))
    })
    .bind(&bind_address)
    .map_err(|e| anyhow::anyhow!("Failed to bind to address {bind_address_clone}: {e}"))?
    .run()
    .await
    .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))
}
