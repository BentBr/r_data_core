#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use log::{debug, info};

use r_data_core::bootstrap::{
    build_api_state, create_cache_manager, create_db_pool, init_logger, verify_license_on_startup,
};
use r_data_core_api::{ApiResponse, ApiStateWrapper};
use r_data_core_core::config::load_app_config;
use r_data_core_persistence::OutboxRepository;

/// 404 handler function
async fn default_404_handler() -> impl actix_web::Responder {
    ApiResponse::<()>::not_found("Resource not found")
}

/// Build the CORS layer from configured origins.
///
/// In production an empty or wildcard origin list is a hard error (fail closed).
/// In non-production we allow any origin so the local http compose setup works.
///
/// # Errors
/// Returns `Err` with a human-readable message when production is configured
/// with an empty or wildcard (`*`) origin list.
fn build_cors(origins: &[String], is_production: bool) -> Result<Cors, String> {
    let base = Cors::default()
        .allow_any_method()
        .allow_any_header()
        .expose_headers(vec!["content-disposition"])
        .max_age(3600);

    let wildcard = origins.iter().any(|o| o == "*");

    if is_production {
        if origins.is_empty() || wildcard {
            return Err(
                "CORS misconfiguration: production requires explicit non-wildcard \
                 CORS_ORIGINS, got empty or '*'"
                    .to_string(),
            );
        }
        Ok(origins
            .iter()
            .fold(base, |cors, origin| cors.allowed_origin(origin)))
    } else if wildcard || origins.is_empty() {
        Ok(base.allow_any_origin())
    } else {
        Ok(origins
            .iter()
            .fold(base, |cors, origin| cors.allowed_origin(origin)))
    }
}

#[actix_web::main]
async fn main() -> r_data_core_core::error::Result<()> {
    // Load configuration
    let config = load_app_config().map_err(|e| {
        r_data_core_core::error::Error::Config(format!(
            "Failed to load application configuration: {e}"
        ))
    })?;
    debug!("Loaded conf: {config:?}");

    // Initialize logger
    init_logger(&config.log.level);

    info!("Starting R Data Core server...");
    info!("Environment: {}", config.environment);
    info!("Log level: {}", config.log.level);
    info!("API docs enabled: {}", config.api.enable_docs);

    // Create database pool
    let pool = create_db_pool(&config).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!(
            "Failed to create database connection pool: {e}"
        ))
    })?;
    if config.outbox_enabled {
        OutboxRepository::ensure_table_exists(&pool).await?;
    }

    info!("Using SQLx migrations (run with 'cargo sqlx migrate run')");

    // Initialize cache manager
    let cache_manager = create_cache_manager(&config).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!(
            "Failed to initialize cache manager with Redis: {e}"
        ))
    })?;

    // Verify license on startup
    verify_license_on_startup(&config, cache_manager.clone()).await;

    // Build API state with all services
    let api_state = build_api_state(&config, pool, cache_manager)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to initialize API state: {e}"))
        })?;

    let app_state = web::Data::new(ApiStateWrapper::new(api_state));

    let bind_address = format!("{}:{}", config.api.host, config.api.port);
    let bind_address_clone = bind_address.clone();
    info!("Starting HTTP server at http://{bind_address}");

    // Validate CORS policy once, before spawning workers (fail closed in prod).
    let cors_origins = config.api.cors_origins.clone();
    let is_production = config.is_production();
    // Validate only; the per-worker closure rebuilds the layer.
    drop(build_cors(&cors_origins, is_production).map_err(r_data_core_core::error::Error::Config)?);

    // Start HTTP server
    HttpServer::new(move || {
        let cors =
            build_cors(&cors_origins, is_production).expect("CORS config validated at startup");

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
    .map_err(|e| {
        r_data_core_core::error::Error::Api(format!(
            "Failed to bind to address {bind_address_clone}: {e}"
        ))
    })?
    .run()
    .await
    .map_err(|e| r_data_core_core::error::Error::Api(format!("HTTP server error: {e}")))
}

#[cfg(test)]
mod cors_tests {
    use super::build_cors;

    #[test]
    fn production_rejects_wildcard_or_empty() {
        assert!(build_cors(&["*".to_string()], true).is_err());
        assert!(build_cors(&[], true).is_err());
    }

    #[test]
    fn production_accepts_explicit_origin() {
        assert!(build_cors(&["https://admin.example.com".to_string()], true).is_ok());
    }

    #[test]
    fn development_allows_wildcard_and_empty() {
        assert!(build_cors(&["*".to_string()], false).is_ok());
        assert!(build_cors(&[], false).is_ok());
        assert!(build_cors(&["http://localhost:3000".to_string()], false).is_ok());
    }
}
