use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

mod api;
mod cache;
mod config;
mod db;
mod entity;
mod error;
mod notification;
mod versioning;
mod workflow;

// These modules will be implemented later
// mod workflow;
// mod versioning;
// mod notification;

use crate::api::{admin, auth, docs, middleware, public, ApiState};
use crate::cache::CacheManager;
use crate::config::AppConfig;
use crate::db::migrations;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables and configure the application
    let config = AppConfig::from_env();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    info!("Starting R Data Core server...");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations
    if let Err(e) = migrations::run_migrations(&pool).await {
        error!("Failed to run migrations: {}", e);
        panic!("Failed to run migrations");
    }

    info!("Database migrations completed successfully");

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

    // Shared application state
    let app_state = web::Data::new(ApiState {
        db_pool: pool,
        jwt_secret: config.api.jwt_secret.clone(),
        cache_manager: cache_manager.clone(),
    });

    let bind_address = format!("{}:{}", config.api.host, config.api.port);
    info!("Starting HTTP server at http://{}", bind_address);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // Enable CORS
            .wrap(actix_cors::Cors::permissive())
            // Configure API routes
            .configure(auth::register_routes)
            .configure(admin::register_routes)
            .configure(public::register_routes)
            .configure(docs::register_routes)
    })
    .bind(bind_address)?
    .run()
    .await
}
