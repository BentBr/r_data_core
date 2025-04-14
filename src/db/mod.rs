mod repository;

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

use crate::config::DatabaseConfig;
use crate::error::{Error, Result};
pub use repository::*;

/// Database connection manager
pub struct Database {
    /// Connection pool to the database
    pub pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout))
            .connect(&config.connection_string)
            .await
            .map_err(Error::Database)?;

        Ok(Self { pool })
    }

    /// Check if the database connection is working
    pub async fn check_connection(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// Get a repository for a specific entity type
    pub fn repository<T>(&self, table_name: &str) -> repository::EntityRepository<T>
    where
        T: Send
            + Sync
            + Unpin
            + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + serde::Serialize
            + serde::de::DeserializeOwned
            + 'static,
    {
        repository::EntityRepository::new(self.pool.clone(), table_name)
    }

    /// Transaction management
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.pool.begin().await.map_err(Error::Database)
    }
}
