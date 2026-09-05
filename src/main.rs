#![deny(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

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
/// In a hardened environment (anything that is not an explicit developer or CI
/// environment — staging included) an empty or wildcard origin list is a hard
/// error. Only developer environments allow any origin, so the local http
/// compose setup keeps working.
///
/// # Errors
/// Returns `Err` with a human-readable message when a hardened environment is
/// configured with an empty or wildcard (`*`) origin list.
fn build_cors(origins: &[String], is_hardened: bool) -> Result<Cors, String> {
    let base = Cors::default()
        .allow_any_method()
        .allow_any_header()
        .expose_headers(vec!["content-disposition"])
        .max_age(3600);

    let wildcard = origins.iter().any(|o| o == "*");

    if is_hardened {
        if origins.is_empty() || wildcard {
            return Err(
                "CORS misconfiguration: this environment requires explicit non-wildcard \
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

    // Validate CORS policy once, before spawning workers (fail closed).
    let cors_origins = config.api.cors_origins.clone();
    let is_hardened = config.is_hardened();
    // Validate only; the per-worker closure rebuilds the layer.
    drop(build_cors(&cors_origins, is_hardened).map_err(r_data_core_core::error::Error::Config)?);

    // Start HTTP server
    HttpServer::new(move || {
        // Validated fail-closed at startup (above) before any worker spawned;
        // actix's `Cors` factory is not `Clone`, so it must be rebuilt per worker.
        #[allow(clippy::expect_used)]
        let cors =
            build_cors(&cors_origins, is_hardened).expect("CORS config validated at startup");

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
    use r_data_core_core::config::AppConfig;

    fn origins(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn hardened_rejects_wildcard_or_empty() {
        assert!(build_cors(&origins(&["*"]), true).is_err());
        assert!(build_cors(&[], true).is_err());
    }

    #[test]
    fn hardened_rejects_a_wildcard_hidden_among_real_origins() {
        // One permissive entry defeats the whole list, so the mix must fail too.
        let result = build_cors(&origins(&["https://admin.example.com", "*"]), true);
        assert!(result.is_err());
    }

    #[test]
    fn hardened_error_explains_what_to_fix() {
        let message = build_cors(&[], true).err().unwrap_or_default();
        assert!(
            message.contains("CORS_ORIGINS"),
            "operator needs to know which var to set, got: {message}"
        );
    }

    #[test]
    fn hardened_accepts_explicit_origins() {
        assert!(build_cors(&origins(&["https://admin.example.com"]), true).is_ok());
        assert!(build_cors(
            &origins(&["https://admin.example.com", "https://ops.example.com"]),
            true
        )
        .is_ok());
    }

    #[test]
    fn relaxed_allows_wildcard_and_empty() {
        assert!(build_cors(&origins(&["*"]), false).is_ok());
        assert!(build_cors(&[], false).is_ok());
        assert!(build_cors(&origins(&["http://localhost:3000"]), false).is_ok());
    }

    /// The gate is driven by `is_hardened`, so staging and an unset `APP_ENV`
    /// get the production policy — this is the regression the fix is about.
    #[test]
    fn only_developer_environments_reach_the_relaxed_branch() {
        for env in ["production", "staging", "preprod", ""] {
            assert!(
                build_cors(&origins(&["*"]), AppConfig::env_is_hardened(env)).is_err(),
                "{env} must reject a wildcard origin list"
            );
        }

        for env in ["development", "dev", "local", "test"] {
            assert!(
                build_cors(&origins(&["*"]), AppConfig::env_is_hardened(env)).is_ok(),
                "{env} is a developer environment and stays permissive"
            );
        }
    }
}
