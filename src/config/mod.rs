use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;

use crate::error::{Error, Result};

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection string
    pub connection_string: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API host
    pub host: String,

    /// API port
    pub port: u16,

    /// Enable SSL/TLS
    pub use_tls: bool,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// JWT token expiration in seconds
    pub jwt_expiration: u64,

    /// Enable documentation
    pub enable_docs: bool,

    /// CORS allowed origins
    pub cors_origins: Vec<String>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,

    /// Cache time-to-live in seconds
    pub ttl: u64,

    /// Maximum cache size (number of items)
    pub max_size: u64,
}

/// Workflow engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Number of worker threads
    pub worker_threads: u32,

    /// Default timeout in seconds
    pub default_timeout: u64,

    /// Max concurrent workflows
    pub max_concurrent: u32,
}

/// Log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level
    pub level: String,

    /// Log to file
    pub file: Option<String>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application environment (development, production, etc.)
    pub environment: String,

    /// Database configuration
    pub database: DatabaseConfig,

    /// API configuration
    pub api: ApiConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Workflow configuration
    pub workflow: WorkflowConfig,

    /// Log configuration
    pub log: LogConfig,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenv().ok();

        let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let database = DatabaseConfig {
            connection_string: env::var("DATABASE_URL")
                .map_err(|_| Error::Config("DATABASE_URL not set".to_string()))?,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            connection_timeout: env::var("DATABASE_CONNECTION_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        };

        let api = ApiConfig {
            host: env::var("API_HOST").unwrap_or_else(|_| "rdatacore.docker".to_string()),
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "8888".to_string())
                .parse()
                .unwrap_or(8888),
            use_tls: env::var("API_USE_TLS")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| Error::Config("JWT_SECRET not set".to_string()))?,
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .unwrap_or(86400),
            enable_docs: env::var("API_ENABLE_DOCS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        };

        let cache = CacheConfig {
            enabled: env::var("CACHE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            ttl: env::var("CACHE_TTL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            max_size: env::var("CACHE_MAX_SIZE")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()
                .unwrap_or(10000),
        };

        let workflow = WorkflowConfig {
            worker_threads: env::var("WORKFLOW_WORKER_THREADS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
            default_timeout: env::var("WORKFLOW_DEFAULT_TIMEOUT")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            max_concurrent: env::var("WORKFLOW_MAX_CONCURRENT")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        };

        let log = LogConfig {
            level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            file: env::var("LOG_FILE").ok(),
        };

        Ok(Self {
            environment,
            database,
            api,
            cache,
            workflow,
            log,
        })
    }
}
